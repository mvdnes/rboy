
pub struct Timer {
	priv divider: u8,
	priv counter: u8,
	priv modulo: u8,
	priv control: u8,
	priv internalcnt: uint,
	interrupt: u8,
}

impl Timer {
	pub fn new() -> Timer {
		Timer {
			divider: 0,
			counter: 0,
			modulo: 0,
			control: 0,
			internalcnt: 0,
			interrupt: 0,
		}
	}

	pub fn rb(&self, a: u16) -> u8 {
		match a {
			0xFF04 => self.divider,
			0xFF05 => self.counter,
			0xFF06 => self.modulo,
			0xFF07 => self.control,
			_ => fail!("Timer does not handler read {:4X}", a),
		}
	}

	pub fn wb(&mut self, a: u16, v: u8) {
		match a {
			0xFF04 => { self.divider = 0; },
			0xFF05 => { self.counter = v; },
			0xFF06 => { self.modulo = v; },
			0xFF07 => { self.control = v; },
			_ => fail!("Timer does not handler write {:4X}", a),
		};
	}

	pub fn step(&mut self, mut ticks: uint) {
		let step = match self.control & 0x03 {
			0 => 256,
			1 => 4,
			2 => 16,
			3 => 64,
			_ => { fail!("There is something very very wrong..."); },
		};

		self.internalcnt %= 256;

		while ticks > 0 {
			ticks -= 1;
			self.internalcnt += 1;
			if self.internalcnt % 64 == 0 { self.divider += 1; };
			if self.control & 0x04 == 0 { continue; };
			if self.internalcnt % step == 0 {
				self.counter += 1;
				if self.counter == 0 {
					self.counter = self.modulo;
					self.interrupt |= 0x04;
				}
			}
		};
	}
}

