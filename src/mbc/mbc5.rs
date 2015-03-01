use std::fs::File;
use mbc::{MBC, ram_size};
use util::handle_io;
use std::path;
use std::io::prelude::*;

pub struct MBC5 {
	rom: Vec<u8>,
	ram: Vec<u8>,
	rombank: usize,
	rambank: usize,
	ram_on: bool,
	savepath: Option<path::PathBuf>,
}

impl MBC5 {
	pub fn new(data: Vec<u8>, file: path::PathBuf) -> Option<MBC5> {
		let subtype = data[0x147];
		let svpath = match subtype {
			0x1B | 0x1E => Some(file.with_extension("gbsave")),
			_ => None,
		};
		let ramsize = match subtype {
			0x1A | 0x1B | 0x1D | 0x1E => ram_size(data[0x149]),
			_ => 0,
		};

		let mut res = MBC5 {
			rom: data,
			ram: ::std::iter::repeat(0u8).take(ramsize).collect(),
			rombank: 1,
			rambank: 0,
			ram_on: false,
			savepath: svpath,
		};
		match res.loadram()
		{
			false => None,
			true => Some(res),
		}
	}

	fn loadram(&mut self) -> bool {
		match self.savepath {
			None => {},
			Some(ref savepath) => if savepath.is_file() {
                let mut data = vec![];
				self.ram = match File::open(&savepath).and_then(|mut f| f.read_to_end(&mut data)) {
					Err(_) => { error!("Could not read RAM"); return false; },
					Ok(..) => data,
				};
			},
		};
		true
	}
}

impl Drop for MBC5 {
	fn drop(&mut self) {
		match self.savepath {
			None => {},
			Some(ref path) =>
			{
				handle_io(File::create(path).and_then(|mut f| f.write_all(&*self.ram)), "Could not write savefile");
			},
		};
	}
}

impl MBC for MBC5 {
	fn readrom(&self, a: u16) -> u8 {
		if a < 0x4000 { self.rom[a as usize] }
		else { self.rom[self.rombank * 0x4000 | ((a as usize) & 0x3FFF)] }
	}
	fn readram(&self, a: u16) -> u8 {
		if !self.ram_on { return 0 }
		self.ram[self.rambank * 0x2000 | ((a as usize) & 0x1FFF)]
	}
	fn writerom(&mut self, a: u16, v: u8) {
		match a {
			0x0000 ... 0x1FFF => self.ram_on = v == 0x0A,
			0x2000 ... 0x2FFF => self.rombank = (self.rombank & 0x100) | (v as usize),
			0x3000 ... 0x3FFF => self.rombank = (self.rombank & 0x0FF) | (((v & 0x1) as usize) << 8),
			0x4000 ... 0x5FFF => self.rambank = (v & 0x0F) as usize,
			0x6000 ... 0x7FFF => { /* ? */ },
			_ => panic!("Could not write to {:04X} (MBC5)", a),
		}
	}
	fn writeram(&mut self, a: u16, v: u8) {
		if self.ram_on == false { return }
		self.ram[self.rambank * 0x2000 | ((a as usize) & 0x1FFF)] = v;
	}
}
