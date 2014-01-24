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
			0x01 => { self.reg.b = self.fetchbyte(mmu); self.reg.c = self.fetchbyte(mmu); 3 },
			0x02 => { mmu.wb(self.reg.bc(), self.reg.a); 2 },
			0x03 => { let w = self.reg.bc(); self.reg.b = (w >> 8) as u8; self.reg.c = (w & 0xFF) as u8; 2 },
			0x04 => { self.reg.b += 1; 1 },
			0x05 => { self.reg.b -= 1; 1 },
			0x06 => { self.reg.b = self.fetchbyte(mmu); 2 },
			0x76 => { self.halted = true; 1 },
			0xCB => { self.callCB(mmu) },
			other=> fail!("Instruction {:2X} is not implemented", other),
		}
	}

	fn callCB(&mut self, mmu: &mut MMU) -> uint {
		match self.fetchword(mmu) {
			other => fail!(" Instruction CB{:2X} is not implemented", other),
		}
	}
}

