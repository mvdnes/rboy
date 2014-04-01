pub struct Registers {
	pub a: u8,
	pub f: u8,
	pub b: u8,
	pub c: u8,
	pub d: u8,
	pub e: u8,
	pub h: u8,
	pub l: u8,
	pub pc: u16,
	pub sp: u16,
}

impl Registers {
	pub fn new() -> Registers {
		Registers {
			a: 0x01,
			f: 0xB0,
			b: 0x00,
			c: 0x13,
			d: 0x00,
			e: 0xD8,
			h: 0x01,
			l: 0x4D,
			pc: 0x0100,
			sp: 0xFFFE,
		}
	}

	pub fn new_cgb() -> Registers {
		let mut res = Registers::new();
		res.a = 0x11;
		return res
	}

	pub fn af(&self) -> u16 {
		(self.a as u16 << 8) | (self.f as u16)
	}
	pub fn bc(&self) -> u16 {
		(self.b as u16 << 8) | (self.c as u16)
	}
	pub fn de(&self) -> u16 {
		(self.d as u16 << 8) | (self.e as u16)
	}
	pub fn hl(&self) -> u16 {
		(self.h as u16 << 8) | (self.l as u16)
	}
	pub fn hld(&mut self) -> u16 {
		let res = self.hl();
		self.sethl(res - 1);
		res
	}
	pub fn hli(&mut self) -> u16 {
		let res = self.hl();
		self.sethl(res + 1);
		res
	}

	pub fn setaf(&mut self, value: u16) {
		self.a = (value >> 8) as u8;
		self.f = (value & 0x00FF) as u8;
	}
	pub fn setbc(&mut self, value: u16) {
		self.b = (value >> 8) as u8;
		self.c = (value & 0x00FF) as u8;
	}
	pub fn setde(&mut self, value: u16) {
		self.d = (value >> 8) as u8;
		self.e = (value & 0x00FF) as u8;
	}
	pub fn sethl(&mut self, value: u16) {
		self.h = (value >> 8) as u8;
		self.l = (value & 0x00FF) as u8;
	}

	pub fn flag(&mut self, mask: u8, set: bool) {
		match set {
			true  => self.f |=  mask,
			false => self.f &= !mask,
		}
	}

	pub fn getflag(&self, mask: u8) -> bool {
		self.f & mask > 0
	}
}

