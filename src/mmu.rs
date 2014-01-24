static WRAM_SIZE: uint = 0x2000;
static ZRAM_SIZE: uint = 0x7F;

pub struct MMU {
	priv wram: ~[u8, ..WRAM_SIZE],
	priv zram: ~[u8, ..ZRAM_SIZE],
	inte: u8,
	intf: u8,
}


impl MMU {
	pub fn new() -> MMU {
		// TODO: init GPU, Timer, Keypad (and others?)
		
		let mut res = MMU {
			wram: ~([0, ..WRAM_SIZE]),
			zram: ~([0, ..ZRAM_SIZE]),
			inte: 0,
			intf: 0,
		};

		res.wb(0xFF05, 0);
		res.wb(0xFF06, 0);
		res.wb(0xFF07, 0);
		res.wb(0xFF40, 0x91);
		res.wb(0xFF42, 0);
		res.wb(0xFF43, 0);
		res.wb(0xFF45, 0);
		res.wb(0xFF47, 0xFC);
		res.wb(0xFF48, 0xFF);
		res.wb(0xFF49, 0xFF);
		res.wb(0xFF4A, 0);
		res.wb(0xFF4B, 0);

		return res;
	}

	pub fn rb(&self, address: u16) -> u8 {
		match address {
			0xC000 .. 0xFDFF => self.wram[address & 0x1FFF],
			0xFF0F => self.inte,
			0xFF80 .. 0xFFFE => self.zram[address - 0xFF80],
			0xFFFF => self.intf,
			_ => fail!("rb not implemented for {:X}", address),
		}
	}

	pub fn rw(&self, address: u16) -> u16 {
		(self.rb(address) as u16) | (self.rb(address + 1) as u16 << 8)
	}

	pub fn wb(&mut self, address: u16, value: u8) {
		match address {
			0xC000 .. 0xFDFF => self.wram[address & 0x1FFF] = value,
			0xFF00 => {}, // Keypad
			0xFF01 .. 0xFF02 => {}, // Serial console
			0xFF04 .. 0xFF07 => {}, // Timer
			0xFF40 .. 0xFF4B => {}, // GPU
			0xFF10 .. 0xFF26 => {}, // Sound
			0xFF0F => self.inte = value,
			0xFF80 .. 0xFFFE => self.zram[address - 0xFF80] = value,
			0xFFFF => self.intf = value,
			_ => fail!("wb not implemented for {:X}", address),
		};
	}

	pub fn ww(&mut self, address: u16, value: u16) {
		self.wb(address, (value & 0xFF) as u8);
		self.wb(address + 1, (value >> 8) as u8);
	}
}

