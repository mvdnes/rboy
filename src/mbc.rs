
pub trait MBC {
	fn readrom(&self, a: u16) -> u8;
	fn readram(&self, a: u16) -> u8;
	fn writerom(&mut self, a: u16, v: u8);
	fn writeram(&mut self, a: u16, v: u8);
}

struct MBC0 {
	priv rom: ~[u8],
}

impl MBC0 {
	pub fn new(data: ~[u8]) -> MBC0 {
		MBC0 { rom: data }
	}
}

struct MBC1 {
	priv rom: ~[u8],
	priv ram: ~[u8],
	priv ram_on: bool,
	priv ram_mode: bool,
	priv rombank: u32,
	priv rambank: u32,
	priv savepath: Option<Path>,
}

impl MBC1 {
	pub fn new(data: ~[u8], file: &Path) -> MBC1 {
		let (svpath, ramsize) = match data[0x147] {
			0x02 => (None, ram_size(data[0x149])),
			0x03 => (Some(file.with_extension("gbsave")), ram_size(data[0x149])),
			_ => (None, 0),
		};

		let mut res = MBC1 {
			rom: data,
			ram: ::std::vec::from_elem(ramsize, 0u8),
			ram_on: false,
			ram_mode: false,
			rombank: 1,
			rambank: 0,
			savepath: svpath,
		};
		res.loadram();
		return res
	}

	fn loadram(&mut self) {
		match self.savepath.clone() {
			None => {},
			Some(savepath) => if savepath.is_file() {
					self.ram = ::std::io::File::open(&savepath).read_to_end();
			},
		};
	}
}

impl Drop for MBC1 {
	fn drop(&mut self) {
		match self.savepath.clone() {
			None => {},
			Some(path) => ::std::io::File::create(&path).write(self.ram),
		};
	}
}

struct MBC3 {
	priv rom: ~[u8],
	priv ram: ~[u8],
	priv rombank: u32,
	priv rambank: u32,
	priv ram_on: bool,
	priv savepath: Option<Path>,
}

impl MBC3 {
	pub fn new(data: ~[u8], file: &Path) -> MBC3 {
		let (svpath, ramsize) = match data[0x147] {
			0x10 | 0x13 => (Some(file.with_extension("gbsave")), ram_size(data[0x149])),
			0x12 => (None, ram_size(data[0x149])),
			_ => (None, 0),
		};
		let mut res = MBC3 {
			rom: data,
			ram: ::std::vec::from_elem(ramsize, 0u8),
			rombank: 1,
			rambank: 0,
			ram_on: false,
			savepath: svpath,
		};
		res.loadram();
		return res
	}

	fn loadram(&mut self) {
		match self.savepath.clone() {
			None => {},
			Some(savepath) => if savepath.is_file() {
					self.ram = ::std::io::File::open(&savepath).read_to_end();
			},
		};
	}
}

impl Drop for MBC3 {
	fn drop(&mut self) {
		match self.savepath.clone() {
			None => {},
			Some(path) => ::std::io::File::create(&path).write(self.ram),
		};
	}
}

pub fn get_mbc(file: &Path) -> ~MBC {
	let data: ~[u8] = ::std::io::File::open(file).read_to_end();
	if data.len() < 0x149 { fail!("Rom size to small"); }
	match data[0x147] {
		0x00 => ~MBC0::new(data) as ~MBC,
		0x01 .. 0x03 => ~MBC1::new(data, file) as ~MBC,
		0x0F .. 0x13 => ~MBC3::new(data, file) as ~MBC,
		m => fail!("Unsupported MBC type: {:02X}", m),
	}
}

fn ram_size(v: u8) -> uint {
	match v {
		1 => 0x800,
		2 => 0x2000,
		3 => 0x8000,
		_ => 0,
	}
}

impl MBC for MBC0 {
	fn readrom(&self, a: u16) -> u8 { self.rom[a] }
	fn readram(&self, _a: u16) -> u8 { 0 }
	fn writerom(&mut self, _a: u16, _v: u8) { () }
	fn writeram(&mut self, _a: u16, _v: u8) { () }
}

impl MBC for MBC1 {
	fn readrom(&self, a: u16) -> u8 {
		if a < 0x4000 { self.rom[a] }
		else { self.rom[self.rombank * 0x4000 | ((a as u32) & 0x3FFF) ] }
	}
	fn readram(&self, a: u16) -> u8 {
		if !self.ram_on { return 0 }
		let rambank = if self.ram_mode { self.rambank } else { 0 };
		self.ram[rambank * 0x2000 | a as u32]
	}

	fn writerom(&mut self, a: u16, v: u8) {
		match a {
			0x0000 .. 0x1FFF => { self.ram_on = (v == 0x0A); },
			0x2000 .. 0x3FFF => {
				self.rombank = (self.rombank & 0x60) | match (v as u32) & 0x1F { 0 => 1, n => n }
			},
			0x4000 .. 0x5FFF => {
				if !self.ram_mode {
					self.rombank = self.rombank & 0x1F | (((v as u32) & 0x03) << 5)
				} else {
					self.rambank = (v as u32) & 0x03;
				}
			},
			0x6000 .. 0x7FFF => { self.ram_mode = (v & 0x01) == 0x01; },
			_ => fail!("Could not write to {:04X} (MBC1)", a),
		}
	}

	fn writeram(&mut self, a: u16, v: u8) {
		if !self.ram_on { return }
		let rambank = if self.ram_mode { self.rambank } else { 0 };
		self.ram[rambank * 0x2000 | a as u32] = v;
	}
}

impl MBC for MBC3 {
	fn readrom(&self, a: u16) -> u8 {
		if a < 0x4000 { self.rom[a] }
		else { self.rom[self.rombank * 0x4000 | ((a as u32) & 0x3FFF)] }
	}
	fn readram(&self, a: u16) -> u8 {
		if !self.ram_on { return 0 }
		if self.rambank <= 3 {
			self.ram[self.rambank * 0x2000 | ((a as u32) & 0x1FFF)]
		} else {
			0 // TODO: RTC
		}
	}
	fn writerom(&mut self, a: u16, v: u8) {
		match a {
			0x0000 .. 0x1FFF => self.ram_on = (v == 0x0A),
			0x2000 .. 0x3FFF => {
				self.rombank = match v & 0x7F { 0 => 1, n => n as u32 }
			},
			0x4000 .. 0x5FFF => self.rambank = v as u32,
			0x6000 .. 0x7FFF => {}, // TODO: RTC
			_ => fail!("Could not write to {:04X} (MBC3)", a),
		}
	}
	fn writeram(&mut self, a: u16, v: u8) {
		if self.ram_on == false { return }
		if self.rambank <= 3 {
			self.ram[self.rambank * 0x2000 | ((a as u32) & 0x1FFF)] = v;
		} else {
			// TODO: RTC
		}
	}
}

