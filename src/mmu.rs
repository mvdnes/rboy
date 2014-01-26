use serial::Serial;
use timer::Timer;
use std::io::File;

static WRAM_SIZE: uint = 0x2000;
//static ZRAM_SIZE: uint = 0x7F;
static ZRAM_SIZE: uint = 0x100;

pub struct MMU {
	priv rom: ~[u8],
	priv ram: ~[u8],
	priv wram: ~[u8, ..WRAM_SIZE],
	priv zram: ~[u8, ..ZRAM_SIZE],
	inte: u8,
	intf: u8,
	priv serial: Serial,
	priv timer: Timer,
	priv mbc: MBC,
	priv rombank: u16,
	priv rambank: u16,
	priv mbcmode: bool,
	priv ramon: bool,
}

enum MBC {
	Direct,
	MBC1,
	Unknown,
}

impl MMU {
	pub fn new(romname: &str) -> MMU {
		let mut res = MMU {
			wram: ~([0, ..WRAM_SIZE]),
			zram: ~([0, ..ZRAM_SIZE]),
			rom: ~[],
			ram: ~[],
			inte: 0,
			intf: 0,
			serial: Serial::new(),
			timer: Timer::new(),
			mbc: Unknown,
			rombank: 1,
			rambank: 0,
			mbcmode: false,
			ramon: false,
		};

		res.rom = File::open(&Path::new(romname)).read_to_end();
		res.setmbc();
		match res.mbc {
			Unknown => { fail!("Unsupported MBC detected"); },
			_ => {},
		};

		res.setram();

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

	fn setmbc(&mut self) {
		self.mbc =
		if self.rom.len() < 0x147 { Unknown }
		else {
			match self.rom[0x147] {
				0x00 => Direct,
				0x01 => MBC1,
				_ => Unknown,
			}
		};
	}

	fn setram(&mut self) {
		if self.rom.len() < 0x149 { fail!("Rom size to small"); };
		let ramsize = match self.rom[0x149] {
			0 => 0,
			1 => 0x800,
			2 => 0x2000,
			3 => 0x8000,
			_ => 0,
		};
		self.ram.grow(ramsize, &0);
	}

	pub fn cycle(&mut self, ticks: uint) {
		self.timer.step(ticks);
		self.intf |= self.timer.interrupt;
		self.timer.interrupt = 0;
	}

	pub fn rb(&self, address: u16) -> u8 {
		match address {
			0x0000 .. 0x3FFF => self.rom[address],
			0x4000 .. 0x7FFF => self.readrombank(address),
			//0x8000 .. 0x9FFF => { 0 }, // VRAM
			0xA000 .. 0xBFFF => self.readram(address),
			0xC000 .. 0xFDFF => self.wram[address & 0x1FFF],
			//0xFF00 => { 0 }, // Keypad
			0xFF01 .. 0xFF02 => self.serial.rb(address),
			0xFF04 .. 0xFF07 => self.timer.rb(address),
			0xFF0F => self.intf,
			//0xFF40 .. 0xFF4B => { 0 }, // GPU
			//0xFF80 .. 0xFFFE => self.zram[address - 0xFF80],
			0xFFFF => self.inte,
			0xFF00 .. 0xFFFE => self.zram[address - 0xFF00],
			_ => fail!("rb not implemented for {:X}", address),
		}
	}

	pub fn rw(&self, address: u16) -> u16 {
		(self.rb(address) as u16) | (self.rb(address + 1) as u16 << 8)
	}

	fn readrombank(&self, address: u16) -> u8 {
		match self.mbc {
			Unknown => { 0 },
			Direct => self.rom[address],
			MBC1 =>   self.rom[(self.rombank * 0x4000) | (address & 0x3FFF)],
		}
	}

	fn ramaddress(&self, address: u16) -> Option<u16> {
		match self.mbc {
			Unknown => None,
			Direct => Some(address),
			MBC1 => if !self.ramon || !self.mbcmode { None } else {
				Some(self.rambank * 0x2000 | address)
			},
		}
	}

	fn readram(&self, address: u16) -> u8 {
		match self.ramaddress(address) {
			None => 0,
			Some(n) => self.ram[n],
		}
	}

	pub fn wb(&mut self, address: u16, value: u8) {
		match address {
			0x0000 .. 0x7FFF => self.writerom(address, value),
			0x8000 .. 0x9FFF => {}, // VRAM
			0xA000 .. 0xBFFF => self.writeram(address, value),
			0xC000 .. 0xFDFF => self.wram[address & 0x1FFF] = value,
			//0xFF00 => {}, // Keypad
			0xFF01 .. 0xFF02 => { self.serial.wb(address, value); }, // Serial console
			0xFF04 .. 0xFF07 => { self.timer.wb(address, value); }, // Timer
			//0xFF40 .. 0xFF4B => {}, // GPU
			0xFF4D => {}, // CGB speed switch
			//0xFF10 .. 0xFF26 => {}, // Sound
			0xFF0F => self.intf = value,
			//0xFF80 .. 0xFFFE => self.zram[address - 0xFF80] = value,
			0xFFFF => self.inte = value,
			0xFF00 .. 0xFFFE => self.zram[address - 0xFF00] = value,
			_ => fail!("wb not implemented for {:X}", address),
		};
	}

	pub fn ww(&mut self, address: u16, value: u16) {
		self.wb(address, (value & 0xFF) as u8);
		self.wb(address + 1, (value >> 8) as u8);
	}

	fn writerom(&mut self, address: u16, value: u8) {
		match self.mbc {
			Unknown => {},
			Direct => {},
			MBC1 => {
				match address {
					0x0000 .. 0x1FFF => { self.ramon = (value == 0xA); },
					0x2000 .. 0x3FFF => {
						self.rombank = (self.rombank & 0x60) | (match value as u16 & 0x1F { 0 => 1, n => n });
					},
					0x4000 .. 0x5FFF => {
						if !self.mbcmode { (self.rombank & 0x1F) | (((value as u16) & 0x03) << 5); }
						else { self.rambank = value as u16 & 0x03; }
					},
					0x6000 .. 0x7FFF => { self.mbcmode = ((value & 1) == 1); }
					_ => { fail!(""); },
				}
			}
		}
	}

	fn writeram(&mut self, address: u16, value: u8) {
		match self.ramaddress(address) {
			None => {},
			Some(n) => self.ram[n] = value,
		};
	}
}

