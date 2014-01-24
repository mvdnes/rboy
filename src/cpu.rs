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
		let res = CPU {
			reg: register::Registers::new(),
			halted: false,
			ime: false,
		};

		res
	}

	pub fn cycle(&mut self, mmu: &mut MMU) -> uint {
		match self.handleinterrupt(mmu) {
			0 => {},
			n => return n,
		};

		if self.halted {
			self.call(mmu, 0)
		} else {
			let instr = self.fetchbyte(mmu);
			self.call(mmu, instr)
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
		if n >= 5 {
			fail!("Invalid interrupt triggered");
		}
		mmu.intf &= !(1 << n);
		self.pushstack(mmu, self.reg.pc);
		self.reg.pc = 0x0040 | ((n as u16) << 3);
		
		return 4
	}

	fn pushstack(&mut self, mmu: &mut MMU, value: u16) {
		fail!("pushstack not implemented");
	}

	fn call(&mut self, mmu: &mut MMU, instr: u8) -> uint {
		fail!("Calling mechanism not implemented");
	}

	fn callCB(&mut self, mmu: &mut MMU) -> uint {
		fail!("CB instruction not implemented");
	}
}
