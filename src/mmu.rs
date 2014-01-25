use self::serial::Serial;
use self::timer::Timer;
use std::io::File;

mod serial;
mod timer;

static WRAM_SIZE: uint = 0x2000;
static ZRAM_SIZE: uint = 0x7F;

pub struct MMU {
	priv rom: ~[u8],
	priv ram: ~[u8],
	priv wram: ~[u8, ..WRAM_SIZE],
	priv zram: ~[u8, ..ZRAM_SIZE],
	inte: u8,
	intf: u8,
	priv serial: Serial,
	priv timer: Timer,
}


impl MMU {
	pub fn new() -> MMU {
		// TODO: init GPU, Timer, Keypad (and others?)
		
		let mut res = MMU {
			wram: ~([0, ..WRAM_SIZE]),
			zram: ~([0, ..ZRAM_SIZE]),
			rom: ~[],
			ram: ~[],
			inte: 0,
			intf: 0,
			serial: Serial::new(),
			timer: Timer::new(),
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


		res.rom = File::open(&Path::new("cpu_instrs.gb")).read_to_end();

		return res;
	}

	pub fn cycle(&mut self, ticks: uint) {
		self.timer.step(ticks);
		self.intf |= self.timer.interrupt;
		self.timer.interrupt = 0;
	}

	pub fn rb(&self, address: u16) -> u8 {
		match address {
			0x0000 .. 0x7FFF => self.readrom(address),
			0xC000 .. 0xFDFF => self.wram[address & 0x1FFF],
			0xFF01 .. 0xFF02 => self.serial.rb(address),
			0xFF04 .. 0xFF07 => self.timer.rb(address),
			0xFF0F => self.inte,
			0xFF80 .. 0xFFFE => self.zram[address - 0xFF80],
			0xFFFF => self.intf,
			_ => fail!("rb not implemented for {:X}", address),
		}
	}

	pub fn rw(&self, address: u16) -> u16 {
		(self.rb(address) as u16) | (self.rb(address + 1) as u16 << 8)
	}

	fn readrom(&self, address: u16) -> u8 {
		// TODO: MBC
		return self.rom[address & 0x7FFF];
	}

	pub fn wb(&mut self, address: u16, value: u8) {
		match address {
			0x0000 .. 0x7FFF => self.writerom(address, value),
			0xC000 .. 0xFDFF => self.wram[address & 0x1FFF] = value,
			0xFF00 => {}, // Keypad
			0xFF01 .. 0xFF02 => { self.serial.wb(address, value); }, // Serial console
			0xFF04 .. 0xFF07 => { self.timer.wb(address, value); }, // Timer
			0xFF40 .. 0xFF4B => {}, // GPU
			0xFF10 .. 0xFF26 => {}, // Sound
			0xFF0F => self.intf = value,
			0xFF80 .. 0xFFFE => self.zram[address - 0xFF80] = value,
			0xFFFF => self.inte = value,
			_ => fail!("wb not implemented for {:X}", address),
		};
	}

	pub fn ww(&mut self, address: u16, value: u16) {
		self.wb(address, (value & 0xFF) as u8);
		self.wb(address + 1, (value >> 8) as u8);
	}

	fn writerom(&mut self, _address: u16, _value: u8) {
		// TODO
	}
}

