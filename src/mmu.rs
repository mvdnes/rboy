use serial::Serial;
use timer::Timer;
use keypad::Keypad;
use gpu::GPU;
use sound::Sound;

static WRAM_SIZE: uint = 0x2000;
static ZRAM_SIZE: uint = 0x7F;

pub struct MMU {
	priv wram: ~[u8, ..WRAM_SIZE],
	priv zram: ~[u8, ..ZRAM_SIZE],
	inte: u8,
	intf: u8,
	serial: Serial,
	timer: Timer,
	keypad: Keypad,
	gpu: GPU,
	sound: Sound,
	priv mbc: ~::mbc::MBC,
}

impl MMU {
	pub fn new(romname: &str) -> MMU {
		let newmbc = ::mbc::get_mbc(&Path::new(romname));

		let mut res = MMU {
			wram: ~([0, ..WRAM_SIZE]),
			zram: ~([0, ..ZRAM_SIZE]),
			inte: 0,
			intf: 0,
			serial: Serial::new(),
			timer: Timer::new(),
			keypad: Keypad::new(),
			gpu: GPU::new(),
			sound: Sound::new(),
			mbc: newmbc,
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

	pub fn cycle(&mut self, ticks: uint) {
		self.timer.cycle(ticks);
		self.intf |= self.timer.interrupt;
		self.timer.interrupt = 0;

		self.intf |= self.keypad.interrupt;
		self.keypad.interrupt = 0;

		self.gpu.cycle(ticks);
		self.intf |= self.gpu.interrupt;
		self.gpu.interrupt = 0;
	}

	pub fn rb(&self, address: u16) -> u8 {
		match address {
			0x0000 .. 0x7FFF => self.mbc.readrom(address),
			0x8000 .. 0x9FFF => self.gpu.rb(address),
			0xA000 .. 0xBFFF => self.mbc.readram(address),
			0xC000 .. 0xFDFF => self.wram[address & 0x1FFF],
			0xFE00 .. 0xFE9F => self.gpu.rb(address),
			0xFF00 => self.keypad.rb(),
			0xFF01 .. 0xFF02 => self.serial.rb(address),
			0xFF04 .. 0xFF07 => self.timer.rb(address),
			0xFF0F => self.intf,
			0xFF10 .. 0xFF26 => self.sound.rb(address),
			0xFF40 .. 0xFF4B => self.gpu.rb(address),
			0xFF80 .. 0xFFFE => self.zram[address & 0x007F],
			0xFFFF => self.inte,
			_ => { warn!("rb not implemented for {:X}", address); 0 },
		}
	}

	pub fn rw(&self, address: u16) -> u16 {
		(self.rb(address) as u16) | (self.rb(address + 1) as u16 << 8)
	}

	pub fn wb(&mut self, address: u16, value: u8) {
		match address {
			0x0000 .. 0x7FFF => self.mbc.writerom(address, value),
			0x8000 .. 0x9FFF => self.gpu.wb(address, value),
			0xA000 .. 0xBFFF => self.mbc.writeram(address, value),
			0xC000 .. 0xFDFF => self.wram[address & 0x1FFF] = value,
			0xFE00 .. 0xFE9F => self.gpu.wb(address, value),
			0xFF00 => self.keypad.wb(value),
			0xFF01 .. 0xFF02 => self.serial.wb(address, value),
			0xFF04 .. 0xFF07 => self.timer.wb(address, value),
			0xFF10 .. 0xFF26 => self.sound.wb(address, value),
			0xFF46 => self.oamdma(value),
			0xFF40 .. 0xFF4B => self.gpu.wb(address, value),
			//0xFF4D => {}, // CGB speed switch
			0xFF0F => self.intf = value,
			0xFF80 .. 0xFFFE => self.zram[address & 0x007F] = value,
			0xFFFF => self.inte = value,
			_ => warn!("wb not implemented for {:X}", address),
		};
	}

	pub fn ww(&mut self, address: u16, value: u16) {
		self.wb(address, (value & 0xFF) as u8);
		self.wb(address + 1, (value >> 8) as u8);
	}

	fn oamdma(&mut self, value: u8) {
		let base = (value as u16) << 8;
		for i in range(0u16, 0xA0) {
			let b = self.rb(base + i);
			self.wb(0xFE00 + i, b);
		}
	}
}
