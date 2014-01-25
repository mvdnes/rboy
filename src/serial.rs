
pub struct Serial {
	enabled: bool,
	priv data: u8,
	priv control: u8,
}

impl Serial {
	pub fn new() -> Serial {
		Serial { enabled: true, data: 0, control: 0 }
	}

	pub fn wb(&mut self, a: u16, v: u8) {
		match a {
			0xFF01 => {
				if self.enabled {
					println!("{}", v as char);
					::std::io::stdio::flush();
				}
				self.data = v;
			},
			0xFF02 => { self.control = v; },
			_ => { fail!("Serial does not handle address {:4X} (write)", a); },
		};
	}

	pub fn rb(&self, a: u16) -> u8 {
		match a {
			0xFF01 => self.data,
			0xFF02 => self.control,
			_ => fail!("Serial does not handle address {:4X} (read)", a),
		}
	}
}
