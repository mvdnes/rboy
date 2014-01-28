
pub struct Sound {
	data: [u8,.. 0x16],
}

impl Sound {
	pub fn new() -> Sound {
		Sound { data: [0,.. 0x16] }
	}

	pub fn rb(&self, a: u16) -> u8 {
		self.data[a - 0xFF10]
	}

	pub fn wb(&mut self, a: u16, v: u8) {
		self.data[a - 0xFF10] = v;
	}
}

