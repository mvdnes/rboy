use std::io::fs::PathExtensions;
use mbc::{MBC, ram_size};
use util::handle_io;

pub struct MBC5 {
	rom: Vec<u8>,
	ram: Vec<u8>,
	rombank: uint,
	rambank: uint,
	ram_on: bool,
	savepath: Option<Path>,
}

impl MBC5 {
	pub fn new(data: Vec<u8>, file: &Path) -> Option<MBC5> {
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
			ram: ::std::vec::Vec::from_elem(ramsize, 0u8),
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
				self.ram = match ::std::io::File::open(savepath).read_to_end() {
					Err(_) => { error!("Could not read RAM"); return false; },
					Ok(data) => data,
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
				handle_io(::std::io::File::create(path).write(self.ram.as_slice()), "Could not write savefile");
			},
		};
	}
}

impl MBC for MBC5 {
	fn readrom(&self, a: u16) -> u8 {
		if a < 0x4000 { self.rom[a as uint] }
		else { self.rom[self.rombank * 0x4000 | ((a as uint) & 0x3FFF)] }
	}
	fn readram(&self, a: u16) -> u8 {
		if !self.ram_on { return 0 }
		self.ram[self.rambank * 0x2000 | ((a as uint) & 0x1FFF)]
	}
	fn writerom(&mut self, a: u16, v: u8) {
		match a {
			0x0000 ... 0x1FFF => self.ram_on = v == 0x0A,
			0x2000 ... 0x2FFF => self.rombank = (self.rombank & 0x100) | (v as uint),
			0x3000 ... 0x3FFF => self.rombank = (self.rombank & 0x0FF) | ((v & 0x1) as uint << 8),
			0x4000 ... 0x5FFF => self.rambank = (v & 0x0F) as uint,
			0x6000 ... 0x7FFF => { /* ? */ },
			_ => fail!("Could not write to {:04X} (MBC5)", a),
		}
	}
	fn writeram(&mut self, a: u16, v: u8) {
		if self.ram_on == false { return }
		*self.ram.get_mut(self.rambank * 0x2000 | ((a as uint) & 0x1FFF)) = v;
	}
}
