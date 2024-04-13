use crate::mbc::{MBC, ram_banks};
use crate::StrResult;

use std::io::prelude::*;
use std::time;
use std::convert::TryInto;

pub struct MBC3 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rombank: usize,
    rambank: usize,
    rambanks: usize,
    selectrtc: bool,
    ram_on: bool,
    ram_updated: bool,
    has_battery: bool,
    rtc_ram: [u8; 5],
    rtc_ram_latch: [u8; 5],
    rtc_zero: Option<u64>,
}

impl MBC3 {
    pub fn new(data: Vec<u8>) -> StrResult<MBC3> {
        let subtype = data[0x147];
        let has_battery = match subtype {
            0x0F | 0x10 | 0x13 => true,
            _ => false,
        };
        let rambanks = match subtype {
            0x10 | 0x12 | 0x13 => ram_banks(data[0x149]),
            _ => 0,
        };
        let ramsize = rambanks * 0x2000;
        let rtc = match subtype {
            0x0F | 0x10 => Some(0),
            _ => None,
        };

        let res = MBC3 {
            rom: data,
            ram: ::std::iter::repeat(0u8).take(ramsize).collect(),
            rombank: 1,
            rambank: 0,
            rambanks: rambanks,
            selectrtc: false,
            ram_on: false,
            ram_updated: false,
            has_battery: has_battery,
            rtc_ram: [0u8; 5],
            rtc_ram_latch: [0u8; 5],
            rtc_zero: rtc,
        };

        Ok(res)
    }

    fn latch_rtc_reg(&mut self) {
        self.calc_rtc_reg();
        self.rtc_ram_latch.clone_from_slice(&self.rtc_ram);
    }

    fn calc_rtc_reg(&mut self) {
        // Do not modify regs when halted
        if self.rtc_ram[4] & 0x40 == 0x40 { return }

        let tzero = match self.rtc_zero {
            Some(t) => time::UNIX_EPOCH + time::Duration::from_secs(t),
            None => return,
        };

        if self.compute_difftime() == self.rtc_zero {
            // No time has passed. Do not alter registers
            return;
        }

        let difftime = match time::SystemTime::now().duration_since(tzero) {
            Ok(n) => { n.as_secs() },
            _ => { 0 },
        };
        self.rtc_ram[0] = (difftime % 60) as u8;
        self.rtc_ram[1] = ((difftime / 60) % 60) as u8;
        self.rtc_ram[2] = ((difftime / 3600) % 24) as u8;
        let days = difftime / (3600*24);
        self.rtc_ram[3] = days as u8;
        self.rtc_ram[4] = (self.rtc_ram[4] & 0xFE) | (((days >> 8) & 0x01) as u8);
        if days >= 512 {
            self.rtc_ram[4] |= 0x80;
            self.calc_rtc_zero();
        }
    }

    fn compute_difftime(&self) -> Option<u64> {
        if self.rtc_zero.is_none() { return None; }
        let mut difftime = match time::SystemTime::now().duration_since(time::UNIX_EPOCH) {
            Ok(t) => t.as_secs(),
            Err(_) => panic!("System clock is set to a time before the unix epoch (1970-01-01)"),
        };
        difftime -= self.rtc_ram[0] as u64;
        difftime -= (self.rtc_ram[1] as u64) * 60;
        difftime -= (self.rtc_ram[2] as u64) * 3600;
        let days = ((self.rtc_ram[4] as u64 & 0x1) << 8) | (self.rtc_ram[3] as u64);
        difftime -= days * 3600 * 24;
        Some(difftime)
    }

    fn calc_rtc_zero(&mut self) {
        self.rtc_zero = self.compute_difftime();
    }
}

impl MBC for MBC3 {
    fn readrom(&self, a: u16) -> u8 {
        let idx = if a < 0x4000 { a as usize }
        else { self.rombank * 0x4000 | ((a as usize) & 0x3FFF) };
        *self.rom.get(idx).unwrap_or(&0xFF)
    }
    fn readram(&self, a: u16) -> u8 {
        if !self.ram_on { return 0xFF }
        if !self.selectrtc && self.rambank < self.rambanks {
            self.ram[self.rambank * 0x2000 | ((a as usize) & 0x1FFF)]
        } else if self.selectrtc && self.rambank < 5 {
            self.rtc_ram_latch[self.rambank]
        } else {
            0xFF
        }
    }
    fn writerom(&mut self, a: u16, v: u8) {
        match a {
            0x0000 ..= 0x1FFF => self.ram_on = (v & 0x0F) == 0x0A,
            0x2000 ..= 0x3FFF => {
                self.rombank = match v & 0x7F { 0 => 1, n => n as usize }
            },
            0x4000 ..= 0x5FFF => {
                self.selectrtc = v & 0x8 == 0x8;
                self.rambank = (v & 0x7) as usize;
            },
            0x6000 ..= 0x7FFF => self.latch_rtc_reg(),
            _ => panic!("Could not write to {:04X} (MBC3)", a),
        }
    }
    fn writeram(&mut self, a: u16, v: u8) {
        if !self.ram_on { return }
        if !self.selectrtc && self.rambank < self.rambanks {
            self.ram[self.rambank * 0x2000 | ((a as usize) & 0x1FFF)] = v;
            self.ram_updated = true;
        } else if self.selectrtc && self.rambank < 5 {
            self.calc_rtc_reg();
            let vmask = match self.rambank {
                0 | 1 => 0x3F,
                2 => 0x1F,
                4 => 0xC1,
                _ => 0xFF,
            };
            self.rtc_ram[self.rambank] = v & vmask;
            self.calc_rtc_zero();
            self.ram_updated = true;
        }
    }

    fn is_battery_backed(&self) -> bool {
        self.has_battery
    }

    fn loadram(&mut self, ramdata: &[u8]) -> StrResult<()> {
        if ramdata.len() != 8 + self.ram.len() {
            return Err("Loaded ram is too small");
        }

        let (int_bytes, rest) = ramdata.split_at(8);
        let rtc = u64::from_be_bytes(int_bytes.try_into().unwrap());
        if self.rtc_zero.is_some() {
            self.rtc_zero = Some(rtc);
        }
        self.ram = rest.to_vec();
        Ok(())
    }

    fn dumpram(&self) -> Vec<u8> {
        let rtc = match self.rtc_zero {
            Some(t) => t,
            None => 0,
        };

        let mut file = vec![];

        let mut ok = true;
        if ok { let rtc_bytes = rtc.to_be_bytes(); ok = file.write_all(&rtc_bytes).is_ok(); };
        if ok { let _ = file.write_all(&*self.ram); };

        file
    }

    fn check_and_reset_ram_updated(&mut self) -> bool {
        let result = self.ram_updated;
        self.ram_updated = false;
        result
    }
}
