use crate::mbc::{MBC, rom_banks};
use crate::StrResult;

pub struct MBC2 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    ram_on: bool,
    ram_updated:bool,
    rombank: usize,
    has_battery: bool,
    rombanks: usize,
}

impl MBC2 {
    pub fn new(data: Vec<u8>) -> StrResult<MBC2> {
        let has_battery = match data[0x147] {
            0x06 => true,
            _ => false,
        };
        let rombanks = rom_banks(data[0x148]);

        let res = MBC2 {
            rom: data,
            ram: vec![0; 512],
            ram_on: false,
            ram_updated: false,
            rombank: 1,
            has_battery: has_battery,
            rombanks: rombanks,
        };

        Ok(res)
    }
}

impl MBC for MBC2 {
    fn readrom(&self, a: u16) -> u8 {
        let bank = if a < 0x4000 {
            0
        }
        else {
            self.rombank
        };
        let idx = bank * 0x4000 | ((a as usize) & 0x3FFF);
        *self.rom.get(idx).unwrap_or(&0xFF)
    }
    fn readram(&self, a: u16) -> u8 {
        if !self.ram_on { return 0xFF }
        self.ram[(a as usize) & 0x1FF] | 0xF0
    }

    fn writerom(&mut self, a: u16, v: u8) {
        match a {
            0x0000 ..= 0x3FFF => {
                if a & 0x100 == 0 {
                    self.ram_on = v & 0xF == 0xA;
                }
                else {
                    self.rombank = match (v as usize) & 0x0F {
                        0 => 1,
                        n => n,
                    } % self.rombanks;
                }
            },
            _ => {},
        }
    }

    fn writeram(&mut self, a: u16, v: u8) {
        if !self.ram_on { return }
        self.ram[(a as usize) & 0x1FF] = v | 0xF0;
        self.ram_updated = true;
    }

    fn is_battery_backed(&self) -> bool {
        self.has_battery
    }

    fn loadram(&mut self, ramdata: &[u8]) -> StrResult<()> {
        if ramdata.len() != self.ram.len() {
            return Err("Loaded RAM has incorrect length");
        }

        self.ram = ramdata.to_vec();

        Ok(())
    }

    fn dumpram(&self) -> Vec<u8> {
        self.ram.to_vec()
    }

    fn check_and_reset_ram_updated(&mut self) -> bool {
        let result = self.ram_updated;
        self.ram_updated = false;
        result
    }
}
