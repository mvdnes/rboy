use std::io::fs::PathExtensions;
use mbc::{MBC, ram_size};
use util::handle_io;

pub struct MBC3 {
	rom: Vec<u8>,
	ram: Vec<u8>,
	rombank: uint,
	rambank: uint,
	ram_on: bool,
	savepath: Option<Path>,
	rtc_ram: [u8,.. 5],
	rtc_lock: bool,
	rtc_zero: Option<i64>,
}

impl MBC3 {
	pub fn new(data: Vec<u8>, file: &Path) -> Option<MBC3> {
		let subtype = data[0x147];
		let svpath = match subtype {
			0x0F | 0x10 | 0x13 => Some(file.with_extension("gbsave")),
			_ => None,
		};
		let ramsize = match subtype {
			0x10 | 0x12 | 0x13 => ram_size(data[0x149]),
			_ => 0,
		};
		let rtc = match subtype {
			0x0F | 0x10 => Some(0),
			_ => None,
		};

		let mut res = MBC3 {
			rom: data,
			ram: ::std::vec::Vec::from_elem(ramsize, 0u8),
			rombank: 1,
			rambank: 0,
			ram_on: false,
			savepath: svpath,
			rtc_ram: [0u8,.. 5],
			rtc_lock: false,
			rtc_zero: rtc,
		};
		match res.loadram()
		{
			false => None,
			true => Some(res),
		}
	}

	fn loadram(&mut self) -> bool {
		match self.savepath {
			None => {},
			Some(ref savepath) => if savepath.is_file() {
				let mut file = ::std::io::File::open(savepath);
				let rtc = match handle_io(file.read_be_i64(), "Could not read RTC") {
					None => { return false; },
					Some(value) => value,
				};
				if self.rtc_zero.is_some() { self.rtc_zero = Some(rtc); }
				self.ram = match handle_io(file.read_to_end(), "Could not read ROM") {
					None => { return false; },
					Some(data) => data,
				};
			},
		};
		true
	}

	fn calc_rtc_reg(&mut self) {
		let tzero = match self.rtc_zero {
			Some(t) => t,
			None => return,
		};
		if self.rtc_ram[4] & 0x40 == 0x40 { return }

		let difftime: i64 = match ::time::get_time().sec - tzero {
			n if n >= 0 => { n },
			_ => { 0 },
		};
		self.rtc_ram[0] = (difftime % 60) as u8;
		self.rtc_ram[1] = ((difftime / 60) % 60) as u8;
		self.rtc_ram[2] = ((difftime / 3600) % 24) as u8;
		let days: i64 = difftime / (3600*24);
		self.rtc_ram[3] = days as u8;
		self.rtc_ram[4] = (self.rtc_ram[4] & 0xFE) | (((days >> 8) & 0x01) as u8);
		if days >= 512 {
			self.rtc_ram[4] |= 0x80;
			self.calc_rtc_zero();
		}
	}

	fn calc_rtc_zero(&mut self) {
		if self.rtc_zero.is_none() { return }
		let mut difftime: i64 = ::time::get_time().sec;
		difftime -= self.rtc_ram[0] as i64;
		difftime -= (self.rtc_ram[1] as i64) * 60;
		difftime -= (self.rtc_ram[2] as i64) * 3600;
		let days = ((self.rtc_ram[4] as i64 & 0x1) << 8) | (self.rtc_ram[3] as i64);
		difftime -= days * 3600 * 24;
		self.rtc_zero = Some(difftime);
	}
}

impl Drop for MBC3 {
	fn drop(&mut self) {
		match self.savepath {
			None => {},
			Some(ref path) => {
				let mut file = ::std::io::File::create(path);
				let rtc = match self.rtc_zero {
					Some(t) => t,
					None => 0,
				};
				let mut ok = true;
				if ok { ok = handle_io(file.write_be_i64(rtc), "Could not write savefile").is_some(); };
				if ok { handle_io(file.write(self.ram.as_slice()), "Could not write savefile"); };
			},
		};
	}
}

impl MBC for MBC3 {
	fn readrom(&self, a: u16) -> u8 {
		if a < 0x4000 { self.rom[a as uint] }
		else { self.rom[self.rombank * 0x4000 | ((a as uint) & 0x3FFF)] }
	}
	fn readram(&self, a: u16) -> u8 {
		if !self.ram_on { return 0 }
		if self.rambank <= 3 {
			self.ram[self.rambank * 0x2000 | ((a as uint) & 0x1FFF)]
		} else {
			self.rtc_ram[self.rambank - 0x08]
		}
	}
	fn writerom(&mut self, a: u16, v: u8) {
		match a {
			0x0000 ... 0x1FFF => self.ram_on = v == 0x0A,
			0x2000 ... 0x3FFF => {
				self.rombank = match v & 0x7F { 0 => 1, n => n as uint }
			},
			0x4000 ... 0x5FFF => self.rambank = v as uint,
			0x6000 ... 0x7FFF => match v {
				0 => self.rtc_lock = false,
				1 => {
					if !self.rtc_lock { self.calc_rtc_reg(); };
					self.rtc_lock = true;
				},
				_ => {},
			},
			_ => panic!("Could not write to {:04X} (MBC3)", a),
		}
	}
	fn writeram(&mut self, a: u16, v: u8) {
		if self.ram_on == false { return }
		if self.rambank <= 3 {
			*self.ram.get_mut(self.rambank * 0x2000 | ((a as uint) & 0x1FFF)) = v;
		} else {
			self.rtc_ram[self.rambank - 0x8] = v;
			self.calc_rtc_zero();
		}
	}
}
