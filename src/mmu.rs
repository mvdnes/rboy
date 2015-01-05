use serial::Serial;
use timer::Timer;
use keypad::Keypad;
use gpu::GPU;
use sound::Sound;
use gbmode::{GbMode, GbSpeed};

const WRAM_SIZE: uint = 0x8000;
const ZRAM_SIZE: uint = 0x7F;

#[deriving(PartialEq)]
enum DMAType {
	NoDMA,
	GDMA,
	HDMA,
}

pub struct MMU<'a> {
	wram: [u8; WRAM_SIZE],
	zram: [u8; ZRAM_SIZE],
	hdma: [u8; 4],
	pub inte: u8,
	pub intf: u8,
	pub serial: Serial<'a>,
	pub timer: Timer,
	pub keypad: Keypad,
	pub gpu: GPU,
	pub sound: Sound,
	hdma_status: DMAType,
	hdma_src: u16,
	hdma_dst: u16,
	hdma_len: u8,
	wrambank: uint,
	mbc: Box<::mbc::MBC+'static>,
	gbmode: GbMode,
	gbspeed: GbSpeed,
	speed_switch_req: bool,
}

impl<'a> MMU<'a> {
	pub fn new(romname: &str, serial_callback: Option<|u8|:'a -> u8>) -> Option<MMU<'a>> {
		let mmu_mbc = match ::mbc::get_mbc(&Path::new(romname))
		{
			Some(mbc) => { mbc },
			None => { return None; },
		};
		let serial = match serial_callback {
			Some(cb) => Serial::new_with_callback(cb),
			None => Serial::new(),
		};
		let mut res = MMU {
			wram: [0; WRAM_SIZE],
			zram: [0; ZRAM_SIZE],
			hdma: [0; 4],
			wrambank: 1,
			inte: 0,
			intf: 0,
			serial: serial,
			timer: Timer::new(),
			keypad: Keypad::new(),
			gpu: GPU::new(),
			sound: Sound::new(),
			mbc: mmu_mbc,
			gbmode: GbMode::Classic,
			gbspeed: GbSpeed::Single,
			speed_switch_req: false,
			hdma_src: 0,
			hdma_dst: 0,
			hdma_status: DMAType::NoDMA,
			hdma_len: 0xFF,
		};
		if res.rb(0x0143) == 0xC0 {
			error!("This game does not work in Classic mode");
			return None;
		}
		res.set_initial();
		Some(res)
	}

	pub fn new_cgb(romname: &str, serial_callback: Option<|u8|:'a -> u8>) -> Option<MMU<'a>> {
		let mmu_mbc = match ::mbc::get_mbc(&Path::new(romname))
		{
			Some(mbc) => { mbc },
			None => { return None; },
		};
		let serial = match serial_callback {
			Some(cb) => Serial::new_with_callback(cb),
			None => Serial::new(),
		};
		let mut res = MMU {
			wram: [0; WRAM_SIZE],
			zram: [0; ZRAM_SIZE],
			wrambank: 1,
			hdma: [0; 4],
			inte: 0,
			intf: 0,
			serial: serial,
			timer: Timer::new(),
			keypad: Keypad::new(),
			gpu: GPU::new_cgb(),
			sound: Sound::new(),
			mbc: mmu_mbc,
			gbmode: GbMode::Color,
			gbspeed: GbSpeed::Single,
			speed_switch_req: false,
			hdma_src: 0,
			hdma_dst: 0,
			hdma_status: DMAType::NoDMA,
			hdma_len: 0xFF,
		};
		res.determine_mode();
		res.set_initial();
		Some(res)
	}

	fn set_initial(&mut self) {
		self.wb(0xFF05, 0);
		self.wb(0xFF06, 0);
		self.wb(0xFF07, 0);
		self.wb(0xFF40, 0x91);
		self.wb(0xFF42, 0);
		self.wb(0xFF43, 0);
		self.wb(0xFF45, 0);
		self.wb(0xFF47, 0xFC);
		self.wb(0xFF48, 0xFF);
		self.wb(0xFF49, 0xFF);
		self.wb(0xFF4A, 0);
		self.wb(0xFF4B, 0);
	}

	fn determine_mode(&mut self) {
		let mode = match self.rb(0x0143) & 0x80 {
			0x80 => GbMode::Color,
			_ => GbMode::ColorAsClassic,
		};
		self.gbmode = mode;
		self.gpu.gbmode = mode;
	}

	pub fn do_cycle(&mut self, cputicks: uint) -> uint {
		let ticks = cputicks + self.perform_vramdma();

		let gputicks = ticks /
			if self.gbspeed == GbSpeed::Single { 1 }
			else { 2 };

		self.timer.do_cycle(ticks);
		self.intf |= self.timer.interrupt;
		self.timer.interrupt = 0;

		self.intf |= self.keypad.interrupt;
		self.keypad.interrupt = 0;

		self.gpu.do_cycle(gputicks);
		self.intf |= self.gpu.interrupt;
		self.gpu.interrupt = 0;

		self.sound.do_cycle(gputicks);

		return gputicks;
	}

	pub fn rb(&self, address: u16) -> u8 {
		match address {
			0x0000 ... 0x7FFF => self.mbc.readrom(address),
			0x8000 ... 0x9FFF => self.gpu.rb(address),
			0xA000 ... 0xBFFF => self.mbc.readram(address),
			0xC000 ... 0xCFFF | 0xE000 ... 0xEFFF => self.wram[address as uint & 0x0FFF],
			0xD000 ... 0xDFFF | 0xF000 ... 0xFDFF => self.wram[(self.wrambank * 0x1000) | address as uint & 0x0FFF],
			0xFE00 ... 0xFE9F => self.gpu.rb(address),
			0xFF00 => self.keypad.rb(),
			0xFF01 ... 0xFF02 => self.serial.rb(address),
			0xFF04 ... 0xFF07 => self.timer.rb(address),
			0xFF0F => self.intf,
			0xFF10 ... 0xFF3F => self.sound.rb(address),
			0xFF4D => (if self.gbspeed == GbSpeed::Double { 0x80 } else { 0 }) | (if self.speed_switch_req { 1 } else { 0 }),
			0xFF40 ... 0xFF4F => self.gpu.rb(address),
			0xFF51 ... 0xFF55 => self.hdma_read(address),
			0xFF68 ... 0xFF6B => self.gpu.rb(address),
			0xFF70 => self.wrambank as u8,
			0xFF80 ... 0xFFFE => self.zram[address as uint & 0x007F],
			0xFFFF => self.inte,
			_ => { warn!("rb not implemented for {:X}", address); 0 },
		}
	}

	pub fn rw(&self, address: u16) -> u16 {
		(self.rb(address) as u16) | ((self.rb(address + 1) as u16) << 8)
	}

	pub fn wb(&mut self, address: u16, value: u8) {
		match address {
			0x0000 ... 0x7FFF => self.mbc.writerom(address, value),
			0x8000 ... 0x9FFF => self.gpu.wb(address, value),
			0xA000 ... 0xBFFF => self.mbc.writeram(address, value),
			0xC000 ... 0xCFFF | 0xE000 ... 0xEFFF => self.wram[address as uint & 0x0FFF] = value,
			0xD000 ... 0xDFFF | 0xF000 ... 0xFDFF => self.wram[(self.wrambank * 0x1000) | (address as uint & 0x0FFF)] = value,
			0xFE00 ... 0xFE9F => self.gpu.wb(address, value),
			0xFF00 => self.keypad.wb(value),
			0xFF01 ... 0xFF02 => self.serial.wb(address, value),
			0xFF04 ... 0xFF07 => self.timer.wb(address, value),
			0xFF10 ... 0xFF3F => self.sound.wb(address, value),
			0xFF46 => self.oamdma(value),
			0xFF4D => if value & 0x1 == 0x1 { self.speed_switch_req = true; },
			0xFF40 ... 0xFF4F => self.gpu.wb(address, value),
			0xFF51 ... 0xFF55 => self.hdma_write(address, value),
			0xFF68 ... 0xFF6B => self.gpu.wb(address, value),
			0xFF0F => self.intf = value,
			0xFF70 => { self.wrambank = match value & 0x7 { 0 => 1, n => n as uint }; },
			0xFF80 ... 0xFFFE => self.zram[address as uint & 0x007F] = value,
			0xFFFF => self.inte = value,
			_ => warn!("wb not implemented for {:X}", address),
		};
	}

	pub fn ww(&mut self, address: u16, value: u16) {
		self.wb(address, (value & 0xFF) as u8);
		self.wb(address + 1, (value >> 8) as u8);
	}

	pub fn switch_speed(&mut self) {
		if self.speed_switch_req {
			info!("Switching speed");
			if self.gbspeed == GbSpeed::Double {
				self.gbspeed = GbSpeed::Single;
			} else {
				self.gbspeed = GbSpeed::Double;
			}
		}
		self.speed_switch_req = false;
	}

	fn oamdma(&mut self, value: u8) {
		let base = (value as u16) << 8;
		for i in range(0u16, 0xA0) {
			let b = self.rb(base + i);
			self.wb(0xFE00 + i, b);
		}
	}

	fn hdma_read(&self, a: u16) -> u8 {
		match a {
			0xFF51 ... 0xFF54 => { self.hdma[(a - 0xFF51) as uint] },
			0xFF55 => self.hdma_len | if self.hdma_status == DMAType::NoDMA { 0x80 } else { 0 },
			_ => panic!("The address {:04X} should not be handled by hdma_read", a),
		}
	}

	fn hdma_write(&mut self, a: u16, v: u8) {
		match a {
			0xFF51 => self.hdma[0] = v,
			0xFF52 => self.hdma[1] = v & 0xF0,
			0xFF53 => self.hdma[2] = v & 0x1F,
			0xFF54 => self.hdma[3] = v & 0xF0,
			0xFF55 => {
				if self.hdma_status == DMAType::HDMA {
					if v & 0x80 == 0 { self.hdma_status = DMAType::NoDMA; };
					return;
				}
				let src = ((self.hdma[0] as u16) << 8) | (self.hdma[1] as u16);
				let dst = ((self.hdma[2] as u16) << 8) | (self.hdma[3] as u16) | 0x8000;
				if !(src <= 0x7FF0 || (src >= 0xA000 && src <= 0xDFF0)) { panic!("HDMA transfer with illegal start address {:04X}", src); }

				self.hdma_src = src;
				self.hdma_dst = dst;
				self.hdma_len = v & 0x7F;

				self.hdma_status =
					if v & 0x80 == 0x80 { DMAType::HDMA }
					else { DMAType::GDMA };
			},
			_ => panic!("The address {:04X} should not be handled by hdma_write", a),
		};
	}

	fn perform_vramdma(&mut self) -> uint {
		match self.hdma_status {
			DMAType::NoDMA => 0,
			DMAType::GDMA => self.perform_gdma(),
			DMAType::HDMA => self.perform_hdma(),
		}
	}

	fn perform_hdma(&mut self) -> uint {
		if self.gpu.may_hdma() == false || self.hdma_len == 0xFF {
			return 0;
		}

		self.perform_vramdma_row();
		if self.hdma_len == 0xFF { self.hdma_status = DMAType::NoDMA; }

		return 0x10 * (if self.gbspeed == GbSpeed::Single { 4 } else { 2 });
	}

	fn perform_gdma(&mut self) -> uint {
		let len = self.hdma_len as uint + 1;
		for _i in range(0, len) {
			self.perform_vramdma_row();
		}

		self.hdma_status = DMAType::NoDMA;
		return len * 0x10 * 2;
	}

	fn perform_vramdma_row(&mut self) {
		for j in range(0u16, 0x10) {
			let b: u8 = self.rb(self.hdma_src + j);
			self.gpu.wb(self.hdma_dst + j, b);
		}
		self.hdma_src += 0x10;
		self.hdma_dst += 0x10;
		self.hdma_len -= 1;
	}
}
