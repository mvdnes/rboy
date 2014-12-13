pub struct Keypad {
	row0: u8,
	row1: u8,
	data: u8,
	pub interrupt: u8,
}

#[deriving(Copy)]
pub enum KeypadKey {
	Right,
	Left,
	Up,
	Down,
	A,
	B,
	Select,
	Start,
}

impl Keypad {
	pub fn new() -> Keypad {
		let mut res = Keypad {
			row0: 0x0F,
			row1: 0x0F,
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
			KeypadKey::Right => self.row1 &= !(1 << 0),
			KeypadKey::Left => self.row1 &= !(1 << 1),
			KeypadKey::Up => self.row1 &= !(1 << 2),
			KeypadKey::Down => self.row1 &= !(1 << 3),
			KeypadKey::A => self.row0 &= !(1 << 0),
			KeypadKey::B => self.row0 &= !(1 << 1),
			KeypadKey::Select => self.row0 &= !(1 << 2),
			KeypadKey::Start => self.row0 &= !(1 << 3),
		}
		self.interrupt |= 0x10;
		self.update();
	}

	pub fn keyup(&mut self, key: KeypadKey) {
		match key {
			KeypadKey::Right => self.row1 |= 1 << 0,
			KeypadKey::Left => self.row1 |= 1 << 1,
			KeypadKey::Up => self.row1 |= 1 << 2,
			KeypadKey::Down => self.row1 |= 1 << 3,
			KeypadKey::A => self.row0 |= 1 << 0,
			KeypadKey::B => self.row0 |= 1 << 1,
			KeypadKey::Select => self.row0 |= 1 << 2,
			KeypadKey::Start => self.row0 |= 1 << 3,
		}
		self.update();
	}
}

#[cfg(test)]
mod test {
	use super::KeypadKey;

	#[test]
	fn keys_row0() {
		let mut keypad = super::Keypad::new();
		let keys0 : [KeypadKey, ..4] = [KeypadKey::A, KeypadKey::B, KeypadKey::Select, KeypadKey::Start];

		for i in range(0u, keys0.len()) {
			keypad.keydown(keys0[i]);

			keypad.wb(0x00);
			assert_eq!(keypad.rb(), 0x00);

			keypad.wb(0x10);
			assert_eq!(keypad.rb(), 0x1F & !(1 << i));

			keypad.wb(0x20);
			assert_eq!(keypad.rb(), 0x2F);

			keypad.wb(0x30);
			assert_eq!(keypad.rb(), 0x3F);

			keypad.keyup(keys0[i]);
		}
	}

	#[test]
	fn keys_row1() {
		let mut keypad = super::Keypad::new();
		let keys1 : [KeypadKey, ..4] = [KeypadKey::Right, KeypadKey::Left, KeypadKey::Up, KeypadKey::Down];

		for i in range(0u, keys1.len()) {
			keypad.keydown(keys1[i]);

			keypad.wb(0x00);
			assert_eq!(keypad.rb(), 0x00);

			keypad.wb(0x10);
			assert_eq!(keypad.rb(), 0x1F);

			keypad.wb(0x20);
			assert_eq!(keypad.rb(), 0x2F & !(1 << i));

			keypad.wb(0x30);
			assert_eq!(keypad.rb(), 0x3F);

			keypad.keyup(keys1[i]);
		}
	}
}
