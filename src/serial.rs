pub struct Serial<'a> {
	data: u8,
	control: u8,
	callback: |u8|:'a -> u8,
}

impl<'a> Serial<'a>
{
	pub fn new_with_callback(cb: |u8|:'a -> u8) -> Serial<'a>
	{
		Serial { data: 0, control: 0, callback: cb, }
	}

	pub fn wb(&mut self, a: u16, v: u8) {
		match a {
			0xFF01 => self.data = v,
			0xFF02 => {
				self.control = v;
				if v == 0x81 {
					self.data = (self.callback)(self.data);
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

	pub fn set_callback(&mut self, cb: |u8|:'static -> u8) {
		self.callback = cb;
	}

	pub fn unset_callback(&mut self) {
		self.callback = |_| { 0 };
	}
}

impl Serial<'static> {
	pub fn new() -> Serial<'static> {
		Serial { data: 0, control: 0, callback: |_| { 0 } }
	}
}
