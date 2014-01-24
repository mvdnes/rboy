pub struct Registers {
	a: u8,
	f: u8,
	b: u8,
	c: u8,
	d: u8,
	e: u8,
	h: u8,
	l: u8,
	pc: u16,
	sp: u16,
}

impl Registers {
	pub fn new() -> Registers {
		let res = Registers {
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
		};

		return res;
	}
}

