use util::handle_io;
use std::io::prelude::*;
use std::fs::File;
use std::path;

mod mbc0;
mod mbc1;
mod mbc3;
mod mbc5;

pub trait MBC {
    fn readrom(&self, a: u16) -> u8;
    fn readram(&self, a: u16) -> u8;
    fn writerom(&mut self, a: u16, v: u8);
    fn writeram(&mut self, a: u16, v: u8);
}

pub fn get_mbc(file: path::PathBuf) -> ::StrResult<Box<MBC+'static>> {
    let mut data = vec![];
    try!(handle_io(File::open(&file).and_then(|mut f| f.read_to_end(&mut data)), "Could not read ROM"));
    if data.len() < 0x150 { return Err("Rom size to small"); }
    try!(check_checksum(&data));
    match data[0x147] {
        0x00 => mbc0::MBC0::new(data).map(|v| Box::new(v) as Box<MBC>),
        0x01 ... 0x03 => mbc1::MBC1::new(data, file).map(|v| Box::new(v) as Box<MBC>),
        0x0F ... 0x13 => mbc3::MBC3::new(data, file).map(|v| Box::new(v) as Box<MBC>),
        0x19 ... 0x1E => mbc5::MBC5::new(data, file).map(|v| Box::new(v) as Box<MBC>),
        _ => { Err("Unsupported MBC type") },
    }
}

fn ram_size(v: u8) -> usize {
    match v {
        1 => 0x800,
        2 => 0x2000,
        3 => 0x8000,
        4 => 0x20000,
        _ => 0,
    }
}

fn check_checksum(data: &Vec<u8>) -> ::StrResult<()> {
    let mut value: u8 = 0;
    for i in (0x134usize .. 0x14D) {
        value = value.wrapping_sub(data[i]).wrapping_sub(1);
    }
    match data[0x14D] == value
    {
        true => Ok(()),
        false => Err("Cartridge checksum is invalid"),
    }
}
