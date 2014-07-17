use mbc::{MBC, ram_size};
use util::handle_io;

pub struct MBC1 {
	rom: Vec<u8>,
	ram: Vec<u8>,
	ram_on: bool,
	ram_mode: bool,
	rombank: uint,
	rambank: uint,
	savepath: Option<Path>,
}

impl MBC1 {
	pub fn new(data: Vec<u8>, file: &Path) -> Option<MBC1> {
		let (svpath, ramsize) = match data[0x147] {
			0x02 => (None, ram_size(data[0x149])),
			0x03 => (Some(file.with_extension("gbsave")), ram_size(data[0x149])),
			_ => (None, 0),
		};

		let mut res = MBC1 {
			rom: data,
			ram: ::std::vec::Vec::from_elem(ramsize, 0u8),
			ram_on: false,
			ram_mode: false,
			rombank: 1,
			rambank: 0,
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
			Some(ref savepath) => if savepath.is_file()
			{
				self.ram = match ::std::io::File::open(savepath).read_to_end()
				{
					Err(_) => { error!("Could not open save file"); return false },
					Ok(data) => data,
				}
			},
		};
		true
	}
}

impl Drop for MBC1 {
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

impl MBC for MBC1 {
	fn readrom(&self, a: u16) -> u8 {
		if a < 0x4000 { self.rom[a as uint] }
		else { self.rom[self.rombank * 0x4000 | ((a as uint) & 0x3FFF)] }
	}
	fn readram(&self, a: u16) -> u8 {
		if !self.ram_on { return 0 }
		let rambank = if self.ram_mode { self.rambank } else { 0 };
		self.ram[(rambank * 0x2000) | ((a & 0x1FFF) as uint)]
	}

	fn writerom(&mut self, a: u16, v: u8) {
		match a {
			0x0000 .. 0x1FFF => { self.ram_on = v == 0x0A; },
			0x2000 .. 0x3FFF => {
				self.rombank = (self.rombank & 0x60) | match (v as uint) & 0x1F { 0 => 1, n => n }
			},
			0x4000 .. 0x5FFF => {
				if !self.ram_mode {
					self.rombank = self.rombank & 0x1F | (((v as uint) & 0x03) << 5)
				} else {
					self.rambank = (v as uint) & 0x03;
				}
			},
			0x6000 .. 0x7FFF => { self.ram_mode = (v & 0x01) == 0x01; },
			_ => fail!("Could not write to {:04X} (MBC1)", a),
		}
	}

	fn writeram(&mut self, a: u16, v: u8) {
		if !self.ram_on { return }
		let rambank = if self.ram_mode { self.rambank } else { 0 };
		*self.ram.get_mut((rambank * 0x2000) | ((a & 0x1FFF) as uint)) = v;
	}
}
