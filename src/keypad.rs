
pub struct Keypad {
	priv row0: u8,
	priv row1: u8,
	priv data: u8,
	interrupt: u8,
}

pub enum KeypadKey {
	Right,
	Left,
	Up,
	Down,
	A,
	B,
	Select,
	Start,
	Poweroff, // Used to poweroff the CPU
}

impl Keypad {
	pub fn new() -> Keypad {
		let mut res = Keypad {
			row0: 0xFF,
			row1: 0xFF,
			data: 0,
			interrupt: 0,
		};
		res.update();
		return res
	}

	pub fn rb(&self) -> u8 {
		self.data
	}

	pub fn wb(&mut self, value: u8) {
		self.data = value;
		self.update();
	}
	
	fn update(&mut self) {
		self.data &= 0x30;
		if self.data & 0x10 == 0x10 { self.data |= self.row0; }
		if self.data & 0x20 == 0x20 { self.data |= self.row1; }
	}

	pub fn keydown(&mut self, key: KeypadKey) {
		match key {
			Right => self.row1 &= !(1 << 0),
			Left => self.row1 &= !(1 << 1),
			Up => self.row1 &= !(1 << 2),
			Down => self.row1 &= !(1 << 3),
			A => self.row0 &= !(1 << 0),
			B => self.row0 &= !(1 << 1),
			Select => self.row0 &= !(1 << 2),
			Start => self.row0 &= !(1 << 3),
			_ => {},
		}
		self.interrupt |= 0x10;
		self.update();
	}

	pub fn keyup(&mut self, key: KeypadKey) {
		match key {
			Right => self.row1 |= (1 << 0),
			Left => self.row1 |= (1 << 1),
			Up => self.row1 |= (1 << 2),
			Down => self.row1 |= (1 << 3),
			A => self.row0 |= (1 << 0),
			B => self.row0 |= (1 << 1),
			Select => self.row0 |= (1 << 2),
			Start => self.row0 |= (1 << 3),
			_ => {},
		}
		self.update();
	}
}
