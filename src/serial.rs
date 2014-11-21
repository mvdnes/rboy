pub struct Serial {
	pub tostdout: bool,
	data: u8,
	control: u8,
}

impl Serial {
	pub fn new() -> Serial {
		Serial { tostdout: false, data: 0, control: 0 }
	}

	pub fn wb(&mut self, a: u16, v: u8) {
		match a {
			0xFF01 => self.data = v,
			0xFF02 => {
				self.control = v;
				if self.tostdout && v == 0x81 {
					print!("{}", self.data as char);
					::std::io::stdio::flush();
				}
			},
			_ => panic!("Serial does not handle address {:4X} (write)", a),
		};
	}

	pub fn rb(&self, a: u16) -> u8 {
		match a {
			0xFF01 => self.data,
			0xFF02 => self.control,
			_ => panic!("Serial does not handle address {:4X} (read)", a),
		}
	}
}
