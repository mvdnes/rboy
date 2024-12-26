use crate::StrResult;
use std::io;
use std::io::prelude::*;
use std::fs::{self, File};
use std::path;

mod mbc0;
mod mbc1;
mod mbc2;
mod mbc3;
mod mbc5;

pub trait MBC : Send {
    fn readrom(&self, a: u16) -> u8;
    fn readram(&self, a: u16) -> u8;
    fn writerom(&mut self, a: u16, v: u8);
    fn writeram(&mut self, a: u16, v: u8);
    fn check_and_reset_ram_updated(&mut self) -> bool;

    fn is_battery_backed(&self) -> bool;
    fn loadram(&mut self, ramdata: &[u8]) -> StrResult<()>;
    fn dumpram(&self) -> Vec<u8>;

    fn romname(&self) -> String {
        const TITLE_START : u16 = 0x134;
        const CGB_FLAG : u16 = 0x143;

        let title_size = match self.readrom(CGB_FLAG) & 0x80 {
            0x80 => 11,
            _ => 16,
        };

        let mut result = String::with_capacity(title_size as usize);

        for i in 0..title_size {
            match self.readrom(TITLE_START + i) {
                0 => break,
                v => result.push(v as char),
            }
        }

        result
    }
}

pub fn get_mbc(data: Vec<u8>, skip_checksum: bool) -> StrResult<Box<dyn MBC+'static>> {
    if data.len() < 0x150 { return Err("Rom size to small"); }
    if !skip_checksum {
        check_checksum(&data)?;
    }
    match data[0x147] {
        0x00 => mbc0::MBC0::new(data).map(|v| Box::new(v) as Box<dyn MBC>),
        0x01 ..= 0x03 => mbc1::MBC1::new(data).map(|v| Box::new(v) as Box<dyn MBC>),
        0x05 ..= 0x06 => mbc2::MBC2::new(data).map(|v| Box::new(v) as Box<dyn MBC>),
        0x0F ..= 0x13 => mbc3::MBC3::new(data).map(|v| Box::new(v) as Box<dyn MBC>),
        0x19 ..= 0x1E => mbc5::MBC5::new(data).map(|v| Box::new(v) as Box<dyn MBC>),
        _ => { Err("Unsupported MBC type") },
    }
}

pub struct FileBackedMBC {
    rampath: path::PathBuf,
    mbc: Box<dyn MBC>,
}

impl FileBackedMBC {
    pub fn new(rompath: path::PathBuf, skip_checksum: bool) -> StrResult<FileBackedMBC> {
        let mut data = vec![];
        File::open(&rompath).and_then(|mut f| f.read_to_end(&mut data)).map_err(|_| "Could not read ROM")?;
        let mut mbc = get_mbc(data, skip_checksum)?;

        let rampath = rompath.with_extension("gbsave");

        if mbc.is_battery_backed() {
            match fs::File::open(&rampath) {
                Ok(mut file) => {
                    let mut ramdata: Vec<u8> = vec![];
                    match file.read_to_end(&mut ramdata) {
                        Err(..) => return Err("Error while reading existing save file"),
                        Ok(..) => { mbc.loadram(&ramdata)?; },
                    }
                },
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => {},
                Err(_) => return Err("Error loading existing save file"),
            }
        }

        Ok(FileBackedMBC { rampath, mbc })
    }
}

// Implement MBC for FileBackedMBC such that the MMU can use this transparently
impl MBC for FileBackedMBC {
    fn readrom(&self, a: u16) -> u8 {
        self.mbc.readrom(a)
    }

    fn readram(&self, a: u16) -> u8 {
        self.mbc.readram(a)
    }

    fn writerom(&mut self, a: u16, v: u8) {
        self.mbc.writerom(a, v)
    }

    fn writeram(&mut self, a: u16, v: u8) {
        self.mbc.writeram(a, v)
    }

    fn is_battery_backed(&self) -> bool {
        self.mbc.is_battery_backed()
    }

    fn loadram(&mut self, ramdata: &[u8]) -> StrResult<()> {
        self.mbc.loadram(ramdata)
    }

    fn dumpram(&self) -> Vec<u8> {
        self.mbc.dumpram()
    }

    fn check_and_reset_ram_updated(&mut self) -> bool {
        self.mbc.check_and_reset_ram_updated()
    }
}


impl Drop for FileBackedMBC {
    fn drop(&mut self) {
        if self.mbc.is_battery_backed() {
            // TODO: error handling
            let mut file = match fs::File::create(&self.rampath) {
                Ok(f) => f,
                Err(..) => return,
            };
            let _ = file.write_all(&self.mbc.dumpram());
        }
    }
}

fn ram_banks(v: u8) -> usize {
    match v {
        1 =>
            // "Listed in various unofficial docs as 2 KiB. However, a 2 KiB RAM chip was never
            // used in a cartridge. The source of this value is unknown."
            // Needed by some test roms. As we only deal in whole banks, just make it 1 8KiB bank.
            1,
        2 => 1,
        3 => 4,
        4 => 16,
        5 => 8,
        _ => 0,
    }
}

fn rom_banks(v: u8) -> usize {
    if v <= 8 {
        2 << v
    }
    else {
        0
    }
}

fn check_checksum(data: &[u8]) -> StrResult<()> {
    let mut value: u8 = 0;
    for i in 0x134 .. 0x14D {
        value = value.wrapping_sub(data[i]).wrapping_sub(1);
    }
    match data[0x14D] == value
    {
        true => Ok(()),
        false => Err("Cartridge checksum is invalid"),
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn checksum_zero() {
        let mut data = vec![0; 0x150];
        data[0x14D] = -(0x14D_i32 - 0x134_i32) as u8;
        super::check_checksum(&data).unwrap();
    }

    #[test]
    fn checksum_ones() {
        let mut data = vec![1; 0x150];
        data[0x14D] = (-(0x14D_i32 - 0x134_i32) * 2) as u8;
        super::check_checksum(&data).unwrap();
    }
}
