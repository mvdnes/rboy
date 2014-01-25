use mmu::MMU;

mod register;
mod mmu;

pub struct CPU {
	reg: register::Registers,
	halted: bool,
	ime: bool,
}

impl CPU {
	pub fn new() -> CPU {
		CPU {
			reg: register::Registers::new(),
			halted: false,
			ime: false,
		}
	}

	pub fn cycle(&mut self, mmu: &mut MMU) -> uint {
		match self.handleinterrupt(mmu) {
			0 => {},
			n => return n,
		};

		if self.halted {
			// Emulate an noop instruction
			1
		} else {
			self.call(mmu)
		}
	}

	fn fetchbyte(&mut self, mmu: &mut MMU) -> u8 {
		let b = mmu.rb(self.reg.pc);
		self.reg.pc += 1;
		b
	}

	fn fetchword(&mut self, mmu: &mut MMU) -> u16 {
		let w = mmu.rw(self.reg.pc);
		self.reg.pc += 2;
		w
	}

	fn handleinterrupt(&mut self, mmu: &mut MMU) -> uint {
		if self.ime == false && self.halted == false { return 0 }
		
		let triggered = mmu.inte & mmu.intf;
		if triggered == 0 { return 0 }

		self.halted = false;
		if self.ime == false { return 0 }
		self.ime = false;

		let n = triggered.trailing_zeros();
		if n >= 5 { fail!("Invalid interrupt triggered"); }
		mmu.intf &= !(1 << n);
		self.pushstack(mmu, self.reg.pc);
		self.reg.pc = 0x0040 | ((n as u16) << 3);
		
		return 4
	}

	fn pushstack(&mut self, mmu: &mut MMU, value: u16) {
		self.reg.sp -= 2;
		mmu.ww(self.reg.sp, value);
	}

	fn popstack(&mut self, mmu: &mut MMU) -> u16 {
		let res = mmu.rw(self.reg.sp);
		self.reg.sp += 2;
		res
	}

	fn call(&mut self, mmu: &mut MMU) -> uint {
		match self.fetchword(mmu) {
			0x00 => { 1 },
			0x01 => { let v = self.fetchword(mmu); self.reg.setbc(v); 3 },
			0x02 => { mmu.wb(self.reg.bc(), self.reg.a); 2 },
			0x03 => { let v = self.reg.bc() + 1; self.reg.setbc(v); 2 },
			0x04 => { self.reg.b += 1; 1 },
			0x05 => { self.reg.b -= 1; 1 },
			0x06 => { self.reg.b = self.fetchbyte(mmu); 2 },
			0x08 => { let a = self.fetchword(mmu); mmu.ww(a, self.reg.sp); 5 },
			0x0A => { self.reg.a = mmu.rb(self.reg.bc()); 2 },
			0x0E => { self.reg.c = self.fetchbyte(mmu); 2 },
			0x11 => { let v = self.fetchword(mmu); self.reg.setde(v); 3 },
			0x12 => { mmu.wb(self.reg.de(), self.reg.a); 2 },
			0x1A => { self.reg.a = mmu.rb(self.reg.de()); 2 },
			0x16 => { self.reg.d = self.fetchbyte(mmu); 2 },
			0x1E => { self.reg.e = self.fetchbyte(mmu); 2 },
			0x21 => { let v = self.fetchword(mmu); self.reg.sethl(v); 3 },
			0x22 => { mmu.wb(self.reg.hli(), self.reg.a); 2 },
			0x26 => { self.reg.h = self.fetchbyte(mmu); 2 },
			0x2A => { self.reg.a = mmu.rb(self.reg.hli()); 2 },
			0x2E => { self.reg.l = self.fetchbyte(mmu); 2 },
			0x31 => { self.reg.sp = self.fetchword(mmu); 3 },
			0x32 => { mmu.wb(self.reg.hld(), self.reg.a); 2 },
			0x36 => { let v = self.fetchbyte(mmu); mmu.wb(self.reg.hl(), v); 3 },
			0x3A => { self.reg.a = mmu.rb(self.reg.hld()); 2 },
			0x3E => { self.reg.a = self.fetchbyte(mmu); 2 },
			0x40 => { 1 },
			0x41 => { self.reg.b = self.reg.c; 1 },
			0x42 => { self.reg.b = self.reg.d; 1 },
			0x43 => { self.reg.b = self.reg.e; 1 },
			0x44 => { self.reg.b = self.reg.h; 1 },
			0x45 => { self.reg.b = self.reg.l; 1 },
			0x46 => { self.reg.b = mmu.rb(self.reg.hl()); 2 },
			0x47 => { self.reg.b = self.reg.a; 1 },
			0x48 => { self.reg.c = self.reg.b; 1 },
			0x49 => { 1 },
			0x4A => { self.reg.c = self.reg.d; 1 },
			0x4B => { self.reg.c = self.reg.e; 1 },
			0x4C => { self.reg.c = self.reg.h; 1 },
			0x4D => { self.reg.c = self.reg.l; 1 },
			0x4E => { self.reg.c = mmu.rb(self.reg.hl()); 2 },
			0x4F => { self.reg.c = self.reg.a; 1 },
			0x50 => { self.reg.d = self.reg.b; 1 },
			0x51 => { self.reg.d = self.reg.c; 1 },
			0x52 => { 1 },
			0x53 => { self.reg.d = self.reg.e; 1 },
			0x54 => { self.reg.d = self.reg.h; 1 },
			0x55 => { self.reg.d = self.reg.l; 1 },
			0x56 => { self.reg.d = mmu.rb(self.reg.hl()); 2 },
			0x57 => { self.reg.d = self.reg.a; 1 },
			0x58 => { self.reg.e = self.reg.b; 1 },
			0x59 => { self.reg.e = self.reg.c; 1 },
			0x5A => { self.reg.e = self.reg.d; 1 },
			0x5B => { 1 },
			0x5C => { self.reg.e = self.reg.h; 1 },
			0x5D => { self.reg.e = self.reg.l; 1 },
			0x5E => { self.reg.e = mmu.rb(self.reg.hl()); 2 },
			0x5F => { self.reg.e = self.reg.a; 1 },
			0x60 => { self.reg.h = self.reg.b; 1 },
			0x61 => { self.reg.h = self.reg.c; 1 },
			0x62 => { self.reg.h = self.reg.d; 1 },
			0x63 => { self.reg.h = self.reg.e; 1 },
			0x64 => { 1 },
			0x65 => { self.reg.h = self.reg.l; 1 },
			0x66 => { self.reg.h = mmu.rb(self.reg.hl()); 2 },
			0x67 => { self.reg.h = self.reg.a; 1 },
			0x68 => { self.reg.l = self.reg.b; 1 },
			0x69 => { self.reg.l = self.reg.c; 1 },
			0x6A => { self.reg.l = self.reg.d; 1 },
			0x6B => { self.reg.l = self.reg.e; 1 },
			0x6C => { self.reg.l = self.reg.h; 1 },
			0x6D => { 1 },
			0x6E => { self.reg.l = mmu.rb(self.reg.hl()); 2 },
			0x6F => { self.reg.l = self.reg.a; 1 },
			0x70 => { mmu.wb(self.reg.hl(), self.reg.b); 2 },
			0x71 => { mmu.wb(self.reg.hl(), self.reg.c); 2 },
			0x72 => { mmu.wb(self.reg.hl(), self.reg.d); 2 },
			0x73 => { mmu.wb(self.reg.hl(), self.reg.e); 2 },
			0x74 => { mmu.wb(self.reg.hl(), self.reg.h); 2 },
			0x75 => { mmu.wb(self.reg.hl(), self.reg.l); 2 },
			0x76 => { self.halted = true; 1 },
			0x77 => { mmu.wb(self.reg.hl(), self.reg.a); 2 },
			0x78 => { self.reg.a = self.reg.b; 1 },
			0x79 => { self.reg.a = self.reg.c; 1 },
			0x7A => { self.reg.a = self.reg.d; 1 },
			0x7B => { self.reg.a = self.reg.e; 1 },
			0x7C => { self.reg.a = self.reg.h; 1 },
			0x7D => { self.reg.a = self.reg.l; 1 },
			0x7E => { self.reg.a = mmu.rb(self.reg.hl()); 2 },
			0x7F => { 1 },
			0xC1 => { let v = self.popstack(mmu); self.reg.setbc(v); 3 },
			0xC5 => { let v = self.reg.bc(); self.pushstack(mmu, v); 4 },
			0xCB => { self.callCB(mmu) },
			0xD1 => { let v = self.popstack(mmu); self.reg.setde(v); 3 },
			0xD5 => { let v = self.reg.de(); self.pushstack(mmu, v); 4 },
			0xE0 => { let a = 0xFF00 + self.fetchbyte(mmu) as u16; mmu.wb(a, self.reg.a); 3 },
			0xE1 => { let v = self.popstack(mmu); self.reg.sethl(v); 3 },
			0xE2 => { mmu.wb(0xFF00 + self.reg.c as u16, self.reg.a); 2 },
			0xE5 => { let v = self.reg.hl(); self.pushstack(mmu, v); 4 },
			0xEA => { let a = self.fetchword(mmu); mmu.wb(a, self.reg.a); 4 },
			0xF0 => { let a = 0xFF00 + self.fetchbyte(mmu) as u16; self.reg.a = mmu.rb(a); 3 },
			0xF1 => { let v = self.popstack(mmu); self.reg.setaf(v); 3 },
			0xF2 => { self.reg.a = mmu.rb(0xFF00 + self.reg.c as u16); 2 },
			0xF5 => { let v = self.reg.af(); self.pushstack(mmu, v); 4 },
			0xF8 => {
				let a = self.reg.sp;
				let b = self.fetchword(mmu) as i8 as i16 as u16;
				self.reg.flag(register::Set(false),
				              register::Set(false),
				              register::Set((a & 0x000F) + (b & 0x000F) > 0x000F),
				              register::Set((a & 0x00FF) + (b & 0x00FF) > 0x00FF));
				self.reg.sethl(a + b);
				3
			}
			0xF9 => { self.reg.sp = self.reg.hl(); 2 },
			0xFA => { let v = self.fetchword(mmu); self.reg.a = mmu.rb(v); 4 },
			other=> fail!("Instruction {:2X} is not implemented", other),
		}
	}

	fn callCB(&mut self, mmu: &mut MMU) -> uint {
		match self.fetchword(mmu) {
			other => fail!(" Instruction CB{:2X} is not implemented", other),
		}
	}
}

