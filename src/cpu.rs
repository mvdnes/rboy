use std::num::Int;
use register::CpuFlag::{C, N, H, Z};
use serial::SerialCallback;

pub struct CPU<'a> {
	reg: ::register::Registers,
	pub mmu: ::mmu::MMU<'a>,
	halted: bool,
	ime: bool,
	setdi: u32,
	setei: u32,
}

impl<'a> CPU<'a> {
	pub fn new(romname: &str, serial_callback: Option<SerialCallback<'a>>) -> Option<CPU<'a>> {
		let cpu_mmu = match ::mmu::MMU::new(romname, serial_callback)
		{
			Some(mmu) => { mmu },
			None => { return None; },
		};
		Some(CPU {
			reg: ::register::Registers::new(),
			halted: false,
			ime: true,
			setdi: 0,
			setei: 0,
			mmu: cpu_mmu,
		})
	}

	pub fn new_cgb(romname: &str, serial_callback: Option<SerialCallback<'a>>) -> Option<CPU<'a>> {
		let cpu_mmu = match ::mmu::MMU::new_cgb(romname, serial_callback)
		{
			Some(mmu) => { mmu },
			None => { return None; },
		};
		Some(CPU {
			reg: ::register::Registers::new_cgb(),
			halted: false,
			ime: true,
			setdi: 0,
			setei: 0,
			mmu: cpu_mmu,
		})
	}

	pub fn do_cycle(&mut self) -> u32 {
		let ticks = self.docycle() * 2;
		return self.mmu.do_cycle(ticks);
	}

	fn docycle(&mut self) -> u32 {
		self.updateime();
		match self.handleinterrupt() {
			0 => {},
			n => return n,
		};

		if self.halted {
			// Emulate an noop instruction
			1
		} else {
			self.call()
		}
	}

	fn fetchbyte(&mut self) -> u8 {
		let b = self.mmu.rb(self.reg.pc);
		self.reg.pc += 1;
		b
	}

	fn fetchword(&mut self) -> u16 {
		let w = self.mmu.rw(self.reg.pc);
		self.reg.pc += 2;
		w
	}

	fn updateime(&mut self) {
		self.setdi = match self.setdi {
			2 => 1,
			1 => { self.ime = false; 0 },
			_ => 0,
		};
		self.setei = match self.setei {
			2 => 1,
			1 => { self.ime = true; 0 },
			_ => 0,
		};
	}

	fn handleinterrupt(&mut self) -> u32 {
		if self.ime == false && self.halted == false { return 0 }

		let triggered = self.mmu.inte & self.mmu.intf;
		if triggered == 0 { return 0 }

		self.halted = false;
		if self.ime == false { return 0 }
		self.ime = false;

		let n = triggered.trailing_zeros() as u32;
		if n >= 5 { panic!("Invalid interrupt triggered"); }
		self.mmu.intf &= !(1 << n);
		let pc = self.reg.pc;
		self.pushstack(pc);
		self.reg.pc = 0x0040 | ((n as u16) << 3);

		return 4
	}

	fn pushstack(&mut self, value: u16) {
		self.reg.sp -= 2;
		self.mmu.ww(self.reg.sp, value);
	}

	fn popstack(&mut self) -> u16 {
		let res = self.mmu.rw(self.reg.sp);
		self.reg.sp += 2;
		res
	}

	fn call(&mut self) -> u32 {
		let opcode = self.fetchbyte();
		let oldregs = self.reg;
		match opcode {
			0x00 => { 1 },
			0x01 => { let v = self.fetchword(); self.reg.setbc(v); 3 },
			0x02 => { self.mmu.wb(self.reg.bc(), self.reg.a); 2 },
			0x03 => { let v = self.reg.bc() + 1; self.reg.setbc(v); 2 },
			0x04 => { self.reg.b = self.alu_inc(oldregs.b); 1 },
			0x05 => { self.reg.b = self.alu_dec(oldregs.b); 1 },
			0x06 => { self.reg.b = self.fetchbyte(); 2 },
			0x07 => { self.reg.a = self.alu_rlc(oldregs.a); self.reg.flag(Z, false); 1 },
			0x08 => { let a = self.fetchword(); self.mmu.ww(a, self.reg.sp); 5 },
			0x09 => { let v = self.reg.bc(); self.alu_add16(v); 2 },
			0x0A => { self.reg.a = self.mmu.rb(self.reg.bc()); 2 },
			0x0B => { let v = self.reg.bc() - 1; self.reg.setbc(v); 2 },
			0x0C => { self.reg.c = self.alu_inc(oldregs.c); 1 },
			0x0D => { self.reg.c = self.alu_dec(oldregs.c); 1 },
			0x0E => { self.reg.c = self.fetchbyte(); 2 },
			0x0F => { self.reg.a = self.alu_rrc(oldregs.a); self.reg.flag(Z, false); 1 },
			0x10 => { self.mmu.switch_speed(); 1 }, // STOP
			0x11 => { let v = self.fetchword(); self.reg.setde(v); 3 },
			0x12 => { self.mmu.wb(self.reg.de(), self.reg.a); 2 },
			0x13 => { let v = self.reg.de() + 1; self.reg.setde(v); 2 },
			0x14 => { self.reg.d = self.alu_inc(oldregs.d); 1 },
			0x15 => { self.reg.d = self.alu_dec(oldregs.d); 1 },
			0x16 => { self.reg.d = self.fetchbyte(); 2 },
			0x17 => { self.reg.a = self.alu_rl(oldregs.a); self.reg.flag(Z, false); 1 },
			0x18 => { self.cpu_jr(); 3 },
			0x19 => { let v = self.reg.de(); self.alu_add16(v); 2 },
			0x1A => { self.reg.a = self.mmu.rb(self.reg.de()); 2 },
			0x1B => { let v = self.reg.de() - 1; self.reg.setde(v); 2 },
			0x1C => { self.reg.e = self.alu_inc(oldregs.e); 1 },
			0x1D => { self.reg.e = self.alu_dec(oldregs.e); 1 },
			0x1E => { self.reg.e = self.fetchbyte(); 2 },
			0x1F => { self.reg.a = self.alu_rr(oldregs.a); self.reg.flag(Z, false); 1 },
			0x20 => { if !self.reg.getflag(Z) { self.cpu_jr(); 3 } else { self.reg.pc += 1; 2 } },
			0x21 => { let v = self.fetchword(); self.reg.sethl(v); 3 },
			0x22 => { self.mmu.wb(self.reg.hli(), self.reg.a); 2 },
			0x23 => { let v = self.reg.hl() + 1; self.reg.sethl(v); 2 },
			0x24 => { self.reg.h = self.alu_inc(oldregs.h); 1 },
			0x25 => { self.reg.h = self.alu_dec(oldregs.h); 1 },
			0x26 => { self.reg.h = self.fetchbyte(); 2 },
			0x27 => { self.alu_daa(); 1 },
			0x28 => { if self.reg.getflag(Z) { self.cpu_jr(); 3 } else { self.reg.pc += 1; 2  } },
			0x29 => { let v = self.reg.hl(); self.alu_add16(v); 2 },
			0x2A => { self.reg.a = self.mmu.rb(self.reg.hli()); 2 },
			0x2B => { let v = self.reg.hl() - 1; self.reg.sethl(v); 2 },
			0x2C => { self.reg.l = self.alu_inc(oldregs.l); 1 },
			0x2D => { self.reg.l = self.alu_dec(oldregs.l); 1 },
			0x2E => { self.reg.l = self.fetchbyte(); 2 },
			0x2F => { self.reg.a = !self.reg.a; self.reg.flag(H, true); self.reg.flag(N, true); 1 },
			0x30 => { if !self.reg.getflag(C) { self.cpu_jr(); 3 } else { self.reg.pc += 1; 2 } },
			0x31 => { self.reg.sp = self.fetchword(); 3 },
			0x32 => { self.mmu.wb(self.reg.hld(), self.reg.a); 2 },
			0x33 => { self.reg.sp += 1; 2 },
			0x34 => { let a = self.reg.hl(); let v = self.mmu.rb(a); let v2 = self.alu_inc(v); self.mmu.wb(a, v2); 3 },
			0x35 => { let a = self.reg.hl(); let v = self.mmu.rb(a); let v2 = self.alu_dec(v); self.mmu.wb(a, v2); 3 },
			0x36 => { let v = self.fetchbyte(); self.mmu.wb(self.reg.hl(), v); 3 },
			0x37 => { self.reg.flag(C, true); self.reg.flag(H, false); self.reg.flag(N, false); 1 },
			0x38 => { if self.reg.getflag(C) { self.cpu_jr(); 3 } else { self.reg.pc += 1; 2  } },
			0x39 => { let v = self.reg.sp; self.alu_add16(v); 2 },
			0x3A => { self.reg.a = self.mmu.rb(self.reg.hld()); 2 },
			0x3B => { self.reg.sp -= 1; 2 },
			0x3C => { self.reg.a = self.alu_inc(oldregs.a); 1 },
			0x3D => { self.reg.a = self.alu_dec(oldregs.a); 1 },
			0x3E => { self.reg.a = self.fetchbyte(); 2 },
			0x3F => { let v = !self.reg.getflag(C); self.reg.flag(C, v); self.reg.flag(H, false); self.reg.flag(N, false); 1 },
			0x40 => { 1 },
			0x41 => { self.reg.b = self.reg.c; 1 },
			0x42 => { self.reg.b = self.reg.d; 1 },
			0x43 => { self.reg.b = self.reg.e; 1 },
			0x44 => { self.reg.b = self.reg.h; 1 },
			0x45 => { self.reg.b = self.reg.l; 1 },
			0x46 => { self.reg.b = self.mmu.rb(self.reg.hl()); 2 },
			0x47 => { self.reg.b = self.reg.a; 1 },
			0x48 => { self.reg.c = self.reg.b; 1 },
			0x49 => { 1 },
			0x4A => { self.reg.c = self.reg.d; 1 },
			0x4B => { self.reg.c = self.reg.e; 1 },
			0x4C => { self.reg.c = self.reg.h; 1 },
			0x4D => { self.reg.c = self.reg.l; 1 },
			0x4E => { self.reg.c = self.mmu.rb(self.reg.hl()); 2 },
			0x4F => { self.reg.c = self.reg.a; 1 },
			0x50 => { self.reg.d = self.reg.b; 1 },
			0x51 => { self.reg.d = self.reg.c; 1 },
			0x52 => { 1 },
			0x53 => { self.reg.d = self.reg.e; 1 },
			0x54 => { self.reg.d = self.reg.h; 1 },
			0x55 => { self.reg.d = self.reg.l; 1 },
			0x56 => { self.reg.d = self.mmu.rb(self.reg.hl()); 2 },
			0x57 => { self.reg.d = self.reg.a; 1 },
			0x58 => { self.reg.e = self.reg.b; 1 },
			0x59 => { self.reg.e = self.reg.c; 1 },
			0x5A => { self.reg.e = self.reg.d; 1 },
			0x5B => { 1 },
			0x5C => { self.reg.e = self.reg.h; 1 },
			0x5D => { self.reg.e = self.reg.l; 1 },
			0x5E => { self.reg.e = self.mmu.rb(self.reg.hl()); 2 },
			0x5F => { self.reg.e = self.reg.a; 1 },
			0x60 => { self.reg.h = self.reg.b; 1 },
			0x61 => { self.reg.h = self.reg.c; 1 },
			0x62 => { self.reg.h = self.reg.d; 1 },
			0x63 => { self.reg.h = self.reg.e; 1 },
			0x64 => { 1 },
			0x65 => { self.reg.h = self.reg.l; 1 },
			0x66 => { self.reg.h = self.mmu.rb(self.reg.hl()); 2 },
			0x67 => { self.reg.h = self.reg.a; 1 },
			0x68 => { self.reg.l = self.reg.b; 1 },
			0x69 => { self.reg.l = self.reg.c; 1 },
			0x6A => { self.reg.l = self.reg.d; 1 },
			0x6B => { self.reg.l = self.reg.e; 1 },
			0x6C => { self.reg.l = self.reg.h; 1 },
			0x6D => { 1 },
			0x6E => { self.reg.l = self.mmu.rb(self.reg.hl()); 2 },
			0x6F => { self.reg.l = self.reg.a; 1 },
			0x70 => { self.mmu.wb(self.reg.hl(), self.reg.b); 2 },
			0x71 => { self.mmu.wb(self.reg.hl(), self.reg.c); 2 },
			0x72 => { self.mmu.wb(self.reg.hl(), self.reg.d); 2 },
			0x73 => { self.mmu.wb(self.reg.hl(), self.reg.e); 2 },
			0x74 => { self.mmu.wb(self.reg.hl(), self.reg.h); 2 },
			0x75 => { self.mmu.wb(self.reg.hl(), self.reg.l); 2 },
			0x76 => { self.halted = true; 1 },
			0x77 => { self.mmu.wb(self.reg.hl(), self.reg.a); 2 },
			0x78 => { self.reg.a = self.reg.b; 1 },
			0x79 => { self.reg.a = self.reg.c; 1 },
			0x7A => { self.reg.a = self.reg.d; 1 },
			0x7B => { self.reg.a = self.reg.e; 1 },
			0x7C => { self.reg.a = self.reg.h; 1 },
			0x7D => { self.reg.a = self.reg.l; 1 },
			0x7E => { self.reg.a = self.mmu.rb(self.reg.hl()); 2 },
			0x7F => { 1 },
			0x80 => { self.alu_add(oldregs.b, false); 1 },
			0x81 => { self.alu_add(oldregs.c, false); 1 },
			0x82 => { self.alu_add(oldregs.d, false); 1 },
			0x83 => { self.alu_add(oldregs.e, false); 1 },
			0x84 => { self.alu_add(oldregs.h, false); 1 },
			0x85 => { self.alu_add(oldregs.l, false); 1 },
			0x86 => { let v = self.mmu.rb(self.reg.hl()); self.alu_add(v, false); 2 },
			0x87 => { self.alu_add(oldregs.a, false); 1 },
			0x88 => { self.alu_add(oldregs.b, true); 1 },
			0x89 => { self.alu_add(oldregs.c, true); 1 },
			0x8A => { self.alu_add(oldregs.d, true); 1 },
			0x8B => { self.alu_add(oldregs.e, true); 1 },
			0x8C => { self.alu_add(oldregs.h, true); 1 },
			0x8D => { self.alu_add(oldregs.l, true); 1 },
			0x8E => { let v = self.mmu.rb(self.reg.hl()); self.alu_add(v, true); 2 },
			0x8F => { self.alu_add(oldregs.a, true); 1 },
			0x90 => { self.alu_sub(oldregs.b, false); 1 },
			0x91 => { self.alu_sub(oldregs.c, false); 1 },
			0x92 => { self.alu_sub(oldregs.d, false); 1 },
			0x93 => { self.alu_sub(oldregs.e, false); 1 },
			0x94 => { self.alu_sub(oldregs.h, false); 1 },
			0x95 => { self.alu_sub(oldregs.l, false); 1 },
			0x96 => { let v = self.mmu.rb(self.reg.hl()); self.alu_sub(v, false); 2 },
			0x97 => { self.alu_sub(oldregs.a, false); 1 },
			0x98 => { self.alu_sub(oldregs.b, true); 1 },
			0x99 => { self.alu_sub(oldregs.c, true); 1 },
			0x9A => { self.alu_sub(oldregs.d, true); 1 },
			0x9B => { self.alu_sub(oldregs.e, true); 1 },
			0x9C => { self.alu_sub(oldregs.h, true); 1 },
			0x9D => { self.alu_sub(oldregs.l, true); 1 },
			0x9E => { let v = self.mmu.rb(self.reg.hl()); self.alu_sub(v, true); 2 },
			0x9F => { self.alu_sub(oldregs.a, true); 1 },
			0xA0 => { self.alu_and(oldregs.b); 1 },
			0xA1 => { self.alu_and(oldregs.c); 1 },
			0xA2 => { self.alu_and(oldregs.d); 1 },
			0xA3 => { self.alu_and(oldregs.e); 1 },
			0xA4 => { self.alu_and(oldregs.h); 1 },
			0xA5 => { self.alu_and(oldregs.l); 1 },
			0xA6 => { let v = self.mmu.rb(self.reg.hl()); self.alu_and(v); 2 },
			0xA7 => { self.alu_and(oldregs.a); 1 },
			0xA8 => { self.alu_xor(oldregs.b); 1 },
			0xA9 => { self.alu_xor(oldregs.c); 1 },
			0xAA => { self.alu_xor(oldregs.d); 1 },
			0xAB => { self.alu_xor(oldregs.e); 1 },
			0xAC => { self.alu_xor(oldregs.h); 1 },
			0xAD => { self.alu_xor(oldregs.l); 1 },
			0xAE => { let v = self.mmu.rb(self.reg.hl()); self.alu_xor(v); 2 },
			0xAF => { self.alu_xor(oldregs.a); 1 },
			0xB0 => { self.alu_or(oldregs.b); 1 },
			0xB1 => { self.alu_or(oldregs.c); 1 },
			0xB2 => { self.alu_or(oldregs.d); 1 },
			0xB3 => { self.alu_or(oldregs.e); 1 },
			0xB4 => { self.alu_or(oldregs.h); 1 },
			0xB5 => { self.alu_or(oldregs.l); 1 },
			0xB6 => { let v = self.mmu.rb(self.reg.hl()); self.alu_or(v); 2 },
			0xB7 => { self.alu_or(oldregs.a); 1 },
			0xB8 => { self.alu_cp(oldregs.b); 1 },
			0xB9 => { self.alu_cp(oldregs.c); 1 },
			0xBA => { self.alu_cp(oldregs.d); 1 },
			0xBB => { self.alu_cp(oldregs.e); 1 },
			0xBC => { self.alu_cp(oldregs.h); 1 },
			0xBD => { self.alu_cp(oldregs.l); 1 },
			0xBE => { let v = self.mmu.rb(self.reg.hl()); self.alu_cp(v); 2 },
			0xBF => { self.alu_cp(oldregs.a); 1 },
			0xC0 => { if !self.reg.getflag(Z) { self.reg.pc = self.popstack(); 5 } else { 2 } },
			0xC1 => { let v = self.popstack(); self.reg.setbc(v); 3 },
			0xC2 => { if !self.reg.getflag(Z) { self.reg.pc = self.fetchword(); 4 } else { self.reg.pc += 2; 3 } },
			0xC3 => { self.reg.pc = self.fetchword(); 4 },
			0xC4 => { if !self.reg.getflag(Z) { self.pushstack(oldregs.pc + 2); self.reg.pc = self.fetchword(); 6 } else { self.reg.pc += 2; 3 } },
			0xC5 => { let v = self.reg.bc(); self.pushstack(v); 4 },
			0xC6 => { let v = self.fetchbyte(); self.alu_add(v, false); 2 },
			0xC7 => { self.pushstack(oldregs.pc); self.reg.pc = 0x00; 4 },
			0xC8 => { if self.reg.getflag(Z) { self.reg.pc = self.popstack(); 5 } else { 2 } },
			0xC9 => { self.reg.pc = self.popstack(); 4 },
			0xCA => { if self.reg.getflag(Z) { self.reg.pc = self.fetchword(); 4 } else { self.reg.pc += 2; 3 } },
			0xCB => { self.call_cb() },
			0xCC => { if self.reg.getflag(Z) { self.pushstack(oldregs.pc + 2); self.reg.pc = self.fetchword(); 6 } else { self.reg.pc += 2; 3 } },
			0xCD => { self.pushstack(oldregs.pc + 2); self.reg.pc = self.fetchword(); 6 },
			0xCE => { let v = self.fetchbyte(); self.alu_add(v, true); 2 },
			0xCF => { self.pushstack(oldregs.pc); self.reg.pc = 0x08; 4 },
			0xD0 => { if !self.reg.getflag(C) { self.reg.pc = self.popstack(); 5 } else { 2 } },
			0xD1 => { let v = self.popstack(); self.reg.setde(v); 3 },
			0xD2 => { if !self.reg.getflag(C) { self.reg.pc = self.fetchword(); 4 } else { self.reg.pc += 2; 3 } },
			0xD4 => { if !self.reg.getflag(C) { self.pushstack(oldregs.pc + 2); self.reg.pc = self.fetchword(); 6 } else { self.reg.pc += 2; 3 } },
			0xD5 => { let v = self.reg.de(); self.pushstack(v); 4 },
			0xD6 => { let v = self.fetchbyte(); self.alu_sub(v, false); 2 },
			0xD7 => { self.pushstack(oldregs.pc); self.reg.pc = 0x10; 4 },
			0xD8 => { if self.reg.getflag(C) { self.reg.pc = self.popstack(); 5 } else { 2 } },
			0xD9 => { self.reg.pc = self.popstack(); self.setei = 1; 4 },
			0xDA => { if self.reg.getflag(C) { self.reg.pc = self.fetchword(); 4 } else { self.reg.pc += 2; 3 } },
			0xDC => { if self.reg.getflag(C) { self.pushstack(oldregs.pc + 2); self.reg.pc = self.fetchword(); 6 } else { self.reg.pc += 2; 3 } },
			0xDE => { let v = self.fetchbyte(); self.alu_sub(v, true); 2 },
			0xDF => { self.pushstack(oldregs.pc); self.reg.pc = 0x18; 4 },
			0xE0 => { let a = 0xFF00 + self.fetchbyte() as u16; self.mmu.wb(a, self.reg.a); 3 },
			0xE1 => { let v = self.popstack(); self.reg.sethl(v); 3 },
			0xE2 => { self.mmu.wb(0xFF00 + self.reg.c as u16, self.reg.a); 2 },
			0xE5 => { let v = self.reg.hl(); self.pushstack(v); 4 },
			0xE6 => { let v = self.fetchbyte(); self.alu_and(v); 2 },
			0xE7 => { self.pushstack(oldregs.pc); self.reg.pc = 0x20; 4 },
			0xE8 => { self.reg.sp = self.alu_add16imm(oldregs.sp); 4 },
			0xE9 => { self.reg.pc = self.reg.hl(); 1 },
			0xEA => { let a = self.fetchword(); self.mmu.wb(a, self.reg.a); 4 },
			0xEE => { let v = self.fetchbyte(); self.alu_xor(v); 2 },
			0xEF => { self.pushstack(oldregs.pc); self.reg.pc = 0x28; 4 },
			0xF0 => { let a = 0xFF00 + self.fetchbyte() as u16; self.reg.a = self.mmu.rb(a); 3 },
			0xF1 => { let v = self.popstack() & 0xFFF0; self.reg.setaf(v); 3 },
			0xF2 => { self.reg.a = self.mmu.rb(0xFF00 + self.reg.c as u16); 2 },
			0xF3 => { self.setdi = 2; 1 },
			0xF5 => { let v = self.reg.af(); self.pushstack(v); 4 },
			0xF6 => { let v = self.fetchbyte(); self.alu_or(v); 2 },
			0xF7 => { self.pushstack(oldregs.pc); self.reg.pc = 0x30; 4 },
			0xF8 => { let r = self.alu_add16imm(oldregs.sp); self.reg.sethl(r); 3 },
			0xF9 => { self.reg.sp = self.reg.hl(); 2 },
			0xFA => { let a = self.fetchword(); self.reg.a = self.mmu.rb(a); 4 },
			0xFB => { self.setei = 2; 1 },
			0xFE => { let v = self.fetchbyte(); self.alu_cp(v); 2 },
			0xFF => { self.pushstack(oldregs.pc); self.reg.pc = 0x38; 4 },
			other=> panic!("Instruction {:2X} is not implemented", other),
		}
	}

	fn call_cb(&mut self) -> u32 {
		let opcode = self.fetchbyte();
		let oldregs = self.reg;
		match opcode {
			0x00 => { self.reg.b = self.alu_rlc(oldregs.b); 2 },
			0x01 => { self.reg.c = self.alu_rlc(oldregs.c); 2 },
			0x02 => { self.reg.d = self.alu_rlc(oldregs.d); 2 },
			0x03 => { self.reg.e = self.alu_rlc(oldregs.e); 2 },
			0x04 => { self.reg.h = self.alu_rlc(oldregs.h); 2 },
			0x05 => { self.reg.l = self.alu_rlc(oldregs.l); 2 },
			0x06 => { let a = self.reg.hl(); let v = self.mmu.rb(a); let v2 = self.alu_rlc(v); self.mmu.wb(a, v2); 4 },
			0x07 => { self.reg.a = self.alu_rlc(oldregs.a); 2 },
			0x08 => { self.reg.b = self.alu_rrc(oldregs.b); 2 },
			0x09 => { self.reg.c = self.alu_rrc(oldregs.c); 2 },
			0x0A => { self.reg.d = self.alu_rrc(oldregs.d); 2 },
			0x0B => { self.reg.e = self.alu_rrc(oldregs.e); 2 },
			0x0C => { self.reg.h = self.alu_rrc(oldregs.h); 2 },
			0x0D => { self.reg.l = self.alu_rrc(oldregs.l); 2 },
			0x0E => { let a = self.reg.hl(); let v = self.mmu.rb(a); let v2 = self.alu_rrc(v); self.mmu.wb(a, v2); 4 },
			0x0F => { self.reg.a = self.alu_rrc(oldregs.a); 2 },
			0x10 => { self.reg.b = self.alu_rl(oldregs.b); 2 },
			0x11 => { self.reg.c = self.alu_rl(oldregs.c); 2 },
			0x12 => { self.reg.d = self.alu_rl(oldregs.d); 2 },
			0x13 => { self.reg.e = self.alu_rl(oldregs.e); 2 },
			0x14 => { self.reg.h = self.alu_rl(oldregs.h); 2 },
			0x15 => { self.reg.l = self.alu_rl(oldregs.l); 2 },
			0x16 => { let a = self.reg.hl(); let v = self.mmu.rb(a); let v2 = self.alu_rl(v); self.mmu.wb(a, v2); 4 },
			0x17 => { self.reg.a = self.alu_rl(oldregs.a); 2 },
			0x18 => { self.reg.b = self.alu_rr(oldregs.b); 2 },
			0x19 => { self.reg.c = self.alu_rr(oldregs.c); 2 },
			0x1A => { self.reg.d = self.alu_rr(oldregs.d); 2 },
			0x1B => { self.reg.e = self.alu_rr(oldregs.e); 2 },
			0x1C => { self.reg.h = self.alu_rr(oldregs.h); 2 },
			0x1D => { self.reg.l = self.alu_rr(oldregs.l); 2 },
			0x1E => { let a = self.reg.hl(); let v = self.mmu.rb(a); let v2 = self.alu_rr(v); self.mmu.wb(a, v2); 4 },
			0x1F => { self.reg.a = self.alu_rr(oldregs.a); 2 },
			0x20 => { self.reg.b = self.alu_sla(oldregs.b); 2 },
			0x21 => { self.reg.c = self.alu_sla(oldregs.c); 2 },
			0x22 => { self.reg.d = self.alu_sla(oldregs.d); 2 },
			0x23 => { self.reg.e = self.alu_sla(oldregs.e); 2 },
			0x24 => { self.reg.h = self.alu_sla(oldregs.h); 2 },
			0x25 => { self.reg.l = self.alu_sla(oldregs.l); 2 },
			0x26 => { let a = self.reg.hl(); let v = self.mmu.rb(a); let v2 = self.alu_sla(v); self.mmu.wb(a, v2); 4 },
			0x27 => { self.reg.a = self.alu_sla(oldregs.a); 2 },
			0x28 => { self.reg.b = self.alu_sra(oldregs.b); 2 },
			0x29 => { self.reg.c = self.alu_sra(oldregs.c); 2 },
			0x2A => { self.reg.d = self.alu_sra(oldregs.d); 2 },
			0x2B => { self.reg.e = self.alu_sra(oldregs.e); 2 },
			0x2C => { self.reg.h = self.alu_sra(oldregs.h); 2 },
			0x2D => { self.reg.l = self.alu_sra(oldregs.l); 2 },
			0x2E => { let a = self.reg.hl(); let v = self.mmu.rb(a); let v2 = self.alu_sra(v); self.mmu.wb(a, v2); 4 },
			0x2F => { self.reg.a = self.alu_sra(oldregs.a); 2 },
			0x30 => { self.reg.b = self.alu_swap(oldregs.b); 2 },
			0x31 => { self.reg.c = self.alu_swap(oldregs.c); 2 },
			0x32 => { self.reg.d = self.alu_swap(oldregs.d); 2 },
			0x33 => { self.reg.e = self.alu_swap(oldregs.e); 2 },
			0x34 => { self.reg.h = self.alu_swap(oldregs.h); 2 },
			0x35 => { self.reg.l = self.alu_swap(oldregs.l); 2 },
			0x36 => { let a = self.reg.hl(); let v = self.mmu.rb(a); let v2 = self.alu_swap(v); self.mmu.wb(a, v2); 4 },
			0x37 => { self.reg.a = self.alu_swap(oldregs.a); 2 },
			0x38 => { self.reg.b = self.alu_srl(oldregs.b); 2 },
			0x39 => { self.reg.c = self.alu_srl(oldregs.c); 2 },
			0x3A => { self.reg.d = self.alu_srl(oldregs.d); 2 },
			0x3B => { self.reg.e = self.alu_srl(oldregs.e); 2 },
			0x3C => { self.reg.h = self.alu_srl(oldregs.h); 2 },
			0x3D => { self.reg.l = self.alu_srl(oldregs.l); 2 },
			0x3E => { let a = self.reg.hl(); let v = self.mmu.rb(a); let v2 = self.alu_srl(v); self.mmu.wb(a, v2); 4 },
			0x3F => { self.reg.a = self.alu_srl(oldregs.a); 2 },
			0x40 => { self.alu_bit(oldregs.b, 0); 2 },
			0x41 => { self.alu_bit(oldregs.c, 0); 2 },
			0x42 => { self.alu_bit(oldregs.d, 0); 2 },
			0x43 => { self.alu_bit(oldregs.e, 0); 2 },
			0x44 => { self.alu_bit(oldregs.h, 0); 2 },
			0x45 => { self.alu_bit(oldregs.l, 0); 2 },
			0x46 => { let v = self.mmu.rb(self.reg.hl()); self.alu_bit(v, 0); 3 },
			0x47 => { self.alu_bit(oldregs.a, 0); 2 },
			0x48 => { self.alu_bit(oldregs.b, 1); 2 },
			0x49 => { self.alu_bit(oldregs.c, 1); 2 },
			0x4A => { self.alu_bit(oldregs.d, 1); 2 },
			0x4B => { self.alu_bit(oldregs.e, 1); 2 },
			0x4C => { self.alu_bit(oldregs.h, 1); 2 },
			0x4D => { self.alu_bit(oldregs.l, 1); 2 },
			0x4E => { let v = self.mmu.rb(self.reg.hl()); self.alu_bit(v, 1); 3 },
			0x4F => { self.alu_bit(oldregs.a, 1); 2 },
			0x50 => { self.alu_bit(oldregs.b, 2); 2 },
			0x51 => { self.alu_bit(oldregs.c, 2); 2 },
			0x52 => { self.alu_bit(oldregs.d, 2); 2 },
			0x53 => { self.alu_bit(oldregs.e, 2); 2 },
			0x54 => { self.alu_bit(oldregs.h, 2); 2 },
			0x55 => { self.alu_bit(oldregs.l, 2); 2 },
			0x56 => { let v = self.mmu.rb(self.reg.hl()); self.alu_bit(v, 2); 3 },
			0x57 => { self.alu_bit(oldregs.a, 2); 2 },
			0x58 => { self.alu_bit(oldregs.b, 3); 2 },
			0x59 => { self.alu_bit(oldregs.c, 3); 2 },
			0x5A => { self.alu_bit(oldregs.d, 3); 2 },
			0x5B => { self.alu_bit(oldregs.e, 3); 2 },
			0x5C => { self.alu_bit(oldregs.h, 3); 2 },
			0x5D => { self.alu_bit(oldregs.l, 3); 2 },
			0x5E => { let v = self.mmu.rb(self.reg.hl()); self.alu_bit(v, 3); 3 },
			0x5F => { self.alu_bit(oldregs.a, 3); 2 },
			0x60 => { self.alu_bit(oldregs.b, 4); 2 },
			0x61 => { self.alu_bit(oldregs.c, 4); 2 },
			0x62 => { self.alu_bit(oldregs.d, 4); 2 },
			0x63 => { self.alu_bit(oldregs.e, 4); 2 },
			0x64 => { self.alu_bit(oldregs.h, 4); 2 },
			0x65 => { self.alu_bit(oldregs.l, 4); 2 },
			0x66 => { let v = self.mmu.rb(self.reg.hl()); self.alu_bit(v, 4); 3 },
			0x67 => { self.alu_bit(oldregs.a, 4); 2 },
			0x68 => { self.alu_bit(oldregs.b, 5); 2 },
			0x69 => { self.alu_bit(oldregs.c, 5); 2 },
			0x6A => { self.alu_bit(oldregs.d, 5); 2 },
			0x6B => { self.alu_bit(oldregs.e, 5); 2 },
			0x6C => { self.alu_bit(oldregs.h, 5); 2 },
			0x6D => { self.alu_bit(oldregs.l, 5); 2 },
			0x6E => { let v = self.mmu.rb(self.reg.hl()); self.alu_bit(v, 5); 3 },
			0x6F => { self.alu_bit(oldregs.a, 5); 2 },
			0x70 => { self.alu_bit(oldregs.b, 6); 2 },
			0x71 => { self.alu_bit(oldregs.c, 6); 2 },
			0x72 => { self.alu_bit(oldregs.d, 6); 2 },
			0x73 => { self.alu_bit(oldregs.e, 6); 2 },
			0x74 => { self.alu_bit(oldregs.h, 6); 2 },
			0x75 => { self.alu_bit(oldregs.l, 6); 2 },
			0x76 => { let v = self.mmu.rb(self.reg.hl()); self.alu_bit(v, 6); 3 },
			0x77 => { self.alu_bit(oldregs.a, 6); 2 },
			0x78 => { self.alu_bit(oldregs.b, 7); 2 },
			0x79 => { self.alu_bit(oldregs.c, 7); 2 },
			0x7A => { self.alu_bit(oldregs.d, 7); 2 },
			0x7B => { self.alu_bit(oldregs.e, 7); 2 },
			0x7C => { self.alu_bit(oldregs.h, 7); 2 },
			0x7D => { self.alu_bit(oldregs.l, 7); 2 },
			0x7E => { let v = self.mmu.rb(self.reg.hl()); self.alu_bit(v, 7); 3 },
			0x7F => { self.alu_bit(oldregs.a, 7); 2 },
			0x80 => { self.reg.b = self.reg.b & !(1 << 0); 2 },
			0x81 => { self.reg.c = self.reg.c & !(1 << 0); 2 },
			0x82 => { self.reg.d = self.reg.d & !(1 << 0); 2 },
			0x83 => { self.reg.e = self.reg.e & !(1 << 0); 2 },
			0x84 => { self.reg.h = self.reg.h & !(1 << 0); 2 },
			0x85 => { self.reg.l = self.reg.l & !(1 << 0); 2 },
			0x86 => { let a = self.reg.hl(); let v = self.mmu.rb(a) & !(1 << 0); self.mmu.wb(a, v); 4 },
			0x87 => { self.reg.a = self.reg.a & !(1 << 0); 2 },
			0x88 => { self.reg.b = self.reg.b & !(1 << 1); 2 },
			0x89 => { self.reg.c = self.reg.c & !(1 << 1); 2 },
			0x8A => { self.reg.d = self.reg.d & !(1 << 1); 2 },
			0x8B => { self.reg.e = self.reg.e & !(1 << 1); 2 },
			0x8C => { self.reg.h = self.reg.h & !(1 << 1); 2 },
			0x8D => { self.reg.l = self.reg.l & !(1 << 1); 2 },
			0x8E => { let a = self.reg.hl(); let v = self.mmu.rb(a) & !(1 << 1); self.mmu.wb(a, v); 4 },
			0x8F => { self.reg.a = self.reg.a & !(1 << 1); 2 },
			0x90 => { self.reg.b = self.reg.b & !(1 << 2); 2 },
			0x91 => { self.reg.c = self.reg.c & !(1 << 2); 2 },
			0x92 => { self.reg.d = self.reg.d & !(1 << 2); 2 },
			0x93 => { self.reg.e = self.reg.e & !(1 << 2); 2 },
			0x94 => { self.reg.h = self.reg.h & !(1 << 2); 2 },
			0x95 => { self.reg.l = self.reg.l & !(1 << 2); 2 },
			0x96 => { let a = self.reg.hl(); let v = self.mmu.rb(a) & !(1 << 2); self.mmu.wb(a, v); 4 },
			0x97 => { self.reg.a = self.reg.a & !(1 << 2); 2 },
			0x98 => { self.reg.b = self.reg.b & !(1 << 3); 2 },
			0x99 => { self.reg.c = self.reg.c & !(1 << 3); 2 },
			0x9A => { self.reg.d = self.reg.d & !(1 << 3); 2 },
			0x9B => { self.reg.e = self.reg.e & !(1 << 3); 2 },
			0x9C => { self.reg.h = self.reg.h & !(1 << 3); 2 },
			0x9D => { self.reg.l = self.reg.l & !(1 << 3); 2 },
			0x9E => { let a = self.reg.hl(); let v = self.mmu.rb(a) & !(1 << 3); self.mmu.wb(a, v); 4 },
			0x9F => { self.reg.a = self.reg.a & !(1 << 3); 2 },
			0xA0 => { self.reg.b = self.reg.b & !(1 << 4); 2 },
			0xA1 => { self.reg.c = self.reg.c & !(1 << 4); 2 },
			0xA2 => { self.reg.d = self.reg.d & !(1 << 4); 2 },
			0xA3 => { self.reg.e = self.reg.e & !(1 << 4); 2 },
			0xA4 => { self.reg.h = self.reg.h & !(1 << 4); 2 },
			0xA5 => { self.reg.l = self.reg.l & !(1 << 4); 2 },
			0xA6 => { let a = self.reg.hl(); let v = self.mmu.rb(a) & !(1 << 4); self.mmu.wb(a, v); 4 },
			0xA7 => { self.reg.a = self.reg.a & !(1 << 4); 2 },
			0xA8 => { self.reg.b = self.reg.b & !(1 << 5); 2 },
			0xA9 => { self.reg.c = self.reg.c & !(1 << 5); 2 },
			0xAA => { self.reg.d = self.reg.d & !(1 << 5); 2 },
			0xAB => { self.reg.e = self.reg.e & !(1 << 5); 2 },
			0xAC => { self.reg.h = self.reg.h & !(1 << 5); 2 },
			0xAD => { self.reg.l = self.reg.l & !(1 << 5); 2 },
			0xAE => { let a = self.reg.hl(); let v = self.mmu.rb(a) & !(1 << 5); self.mmu.wb(a, v); 4 },
			0xAF => { self.reg.a = self.reg.a & !(1 << 5); 2 },
			0xB0 => { self.reg.b = self.reg.b & !(1 << 6); 2 },
			0xB1 => { self.reg.c = self.reg.c & !(1 << 6); 2 },
			0xB2 => { self.reg.d = self.reg.d & !(1 << 6); 2 },
			0xB3 => { self.reg.e = self.reg.e & !(1 << 6); 2 },
			0xB4 => { self.reg.h = self.reg.h & !(1 << 6); 2 },
			0xB5 => { self.reg.l = self.reg.l & !(1 << 6); 2 },
			0xB6 => { let a = self.reg.hl(); let v = self.mmu.rb(a) & !(1 << 6); self.mmu.wb(a, v); 4 },
			0xB7 => { self.reg.a = self.reg.a & !(1 << 6); 2 },
			0xB8 => { self.reg.b = self.reg.b & !(1 << 7); 2 },
			0xB9 => { self.reg.c = self.reg.c & !(1 << 7); 2 },
			0xBA => { self.reg.d = self.reg.d & !(1 << 7); 2 },
			0xBB => { self.reg.e = self.reg.e & !(1 << 7); 2 },
			0xBC => { self.reg.h = self.reg.h & !(1 << 7); 2 },
			0xBD => { self.reg.l = self.reg.l & !(1 << 7); 2 },
			0xBE => { let a = self.reg.hl(); let v = self.mmu.rb(a) & !(1 << 7); self.mmu.wb(a, v); 4 },
			0xBF => { self.reg.a = self.reg.a & !(1 << 7); 2 },
			0xC0 => { self.reg.b = self.reg.b | (1 << 0); 2 },
			0xC1 => { self.reg.c = self.reg.c | (1 << 0); 2 },
			0xC2 => { self.reg.d = self.reg.d | (1 << 0); 2 },
			0xC3 => { self.reg.e = self.reg.e | (1 << 0); 2 },
			0xC4 => { self.reg.h = self.reg.h | (1 << 0); 2 },
			0xC5 => { self.reg.l = self.reg.l | (1 << 0); 2 },
			0xC6 => { let a = self.reg.hl(); let v = self.mmu.rb(a) | (1 << 0); self.mmu.wb(a, v); 4 },
			0xC7 => { self.reg.a = self.reg.a | (1 << 0); 2 },
			0xC8 => { self.reg.b = self.reg.b | (1 << 1); 2 },
			0xC9 => { self.reg.c = self.reg.c | (1 << 1); 2 },
			0xCA => { self.reg.d = self.reg.d | (1 << 1); 2 },
			0xCB => { self.reg.e = self.reg.e | (1 << 1); 2 },
			0xCC => { self.reg.h = self.reg.h | (1 << 1); 2 },
			0xCD => { self.reg.l = self.reg.l | (1 << 1); 2 },
			0xCE => { let a = self.reg.hl(); let v = self.mmu.rb(a) | (1 << 1); self.mmu.wb(a, v); 4 },
			0xCF => { self.reg.a = self.reg.a | (1 << 1); 2 },
			0xD0 => { self.reg.b = self.reg.b | (1 << 2); 2 },
			0xD1 => { self.reg.c = self.reg.c | (1 << 2); 2 },
			0xD2 => { self.reg.d = self.reg.d | (1 << 2); 2 },
			0xD3 => { self.reg.e = self.reg.e | (1 << 2); 2 },
			0xD4 => { self.reg.h = self.reg.h | (1 << 2); 2 },
			0xD5 => { self.reg.l = self.reg.l | (1 << 2); 2 },
			0xD6 => { let a = self.reg.hl(); let v = self.mmu.rb(a) | (1 << 2); self.mmu.wb(a, v); 4 },
			0xD7 => { self.reg.a = self.reg.a | (1 << 2); 2 },
			0xD8 => { self.reg.b = self.reg.b | (1 << 3); 2 },
			0xD9 => { self.reg.c = self.reg.c | (1 << 3); 2 },
			0xDA => { self.reg.d = self.reg.d | (1 << 3); 2 },
			0xDB => { self.reg.e = self.reg.e | (1 << 3); 2 },
			0xDC => { self.reg.h = self.reg.h | (1 << 3); 2 },
			0xDD => { self.reg.l = self.reg.l | (1 << 3); 2 },
			0xDE => { let a = self.reg.hl(); let v = self.mmu.rb(a) | (1 << 3); self.mmu.wb(a, v); 4 },
			0xDF => { self.reg.a = self.reg.a | (1 << 3); 2 },
			0xE0 => { self.reg.b = self.reg.b | (1 << 4); 2 },
			0xE1 => { self.reg.c = self.reg.c | (1 << 4); 2 },
			0xE2 => { self.reg.d = self.reg.d | (1 << 4); 2 },
			0xE3 => { self.reg.e = self.reg.e | (1 << 4); 2 },
			0xE4 => { self.reg.h = self.reg.h | (1 << 4); 2 },
			0xE5 => { self.reg.l = self.reg.l | (1 << 4); 2 },
			0xE6 => { let a = self.reg.hl(); let v = self.mmu.rb(a) | (1 << 4); self.mmu.wb(a, v); 4 },
			0xE7 => { self.reg.a = self.reg.a | (1 << 4); 2 },
			0xE8 => { self.reg.b = self.reg.b | (1 << 5); 2 },
			0xE9 => { self.reg.c = self.reg.c | (1 << 5); 2 },
			0xEA => { self.reg.d = self.reg.d | (1 << 5); 2 },
			0xEB => { self.reg.e = self.reg.e | (1 << 5); 2 },
			0xEC => { self.reg.h = self.reg.h | (1 << 5); 2 },
			0xED => { self.reg.l = self.reg.l | (1 << 5); 2 },
			0xEE => { let a = self.reg.hl(); let v = self.mmu.rb(a) | (1 << 5); self.mmu.wb(a, v); 4 },
			0xEF => { self.reg.a = self.reg.a | (1 << 5); 2 },
			0xF0 => { self.reg.b = self.reg.b | (1 << 6); 2 },
			0xF1 => { self.reg.c = self.reg.c | (1 << 6); 2 },
			0xF2 => { self.reg.d = self.reg.d | (1 << 6); 2 },
			0xF3 => { self.reg.e = self.reg.e | (1 << 6); 2 },
			0xF4 => { self.reg.h = self.reg.h | (1 << 6); 2 },
			0xF5 => { self.reg.l = self.reg.l | (1 << 6); 2 },
			0xF6 => { let a = self.reg.hl(); let v = self.mmu.rb(a) | (1 << 6); self.mmu.wb(a, v); 4 },
			0xF7 => { self.reg.a = self.reg.a | (1 << 6); 2 },
			0xF8 => { self.reg.b = self.reg.b | (1 << 7); 2 },
			0xF9 => { self.reg.c = self.reg.c | (1 << 7); 2 },
			0xFA => { self.reg.d = self.reg.d | (1 << 7); 2 },
			0xFB => { self.reg.e = self.reg.e | (1 << 7); 2 },
			0xFC => { self.reg.h = self.reg.h | (1 << 7); 2 },
			0xFD => { self.reg.l = self.reg.l | (1 << 7); 2 },
			0xFE => { let a = self.reg.hl(); let v = self.mmu.rb(a) | (1 << 7); self.mmu.wb(a, v); 4 },
			0xFF => { self.reg.a = self.reg.a | (1 << 7); 2 },
			other => panic!(" Instruction CB{:2X} is not implemented", other),
		}
	}

	fn alu_add(&mut self, b: u8, usec: bool) {
		let c = if usec && self.reg.getflag(C) { 1 } else { 0 };
		let a = self.reg.a;
		let r = a + b + c;
		self.reg.flag(Z, r == 0);
		self.reg.flag(H, (a & 0xF) + (b & 0xF) + c > 0xF);
		self.reg.flag(N, false);
		self.reg.flag(C, (a as u16) + (b as u16) + (c as u16) > 0xFF);
		self.reg.a = r;
	}

	fn alu_sub(&mut self, b: u8, usec: bool) {
		let c = if usec && self.reg.getflag(C) { 1 } else { 0 };
		let a = self.reg.a;
		let r = a - b - c;
		self.reg.flag(Z, r == 0);
		self.reg.flag(H, (a & 0x0F) < (b & 0x0F) + c);
		self.reg.flag(N, true);
		self.reg.flag(C, (a as u16) < (b as u16) + (c as u16));
		self.reg.a = r;
	}

	fn alu_and(&mut self, b: u8) {
		let r = self.reg.a & b;
		self.reg.flag(Z, r == 0);
		self.reg.flag(H, true);
		self.reg.flag(C, false);
		self.reg.flag(N, false);
		self.reg.a = r;
	}

	fn alu_or(&mut self, b: u8) {
		let r = self.reg.a | b;
		self.reg.flag(Z, r == 0);
		self.reg.flag(C, false);
		self.reg.flag(H, false);
		self.reg.flag(N, false);
		self.reg.a = r;
	}

	fn alu_xor(&mut self, b: u8) {
		let r = self.reg.a ^ b;
		self.reg.flag(Z, r == 0);
		self.reg.flag(C, false);
		self.reg.flag(H, false);
		self.reg.flag(N, false);
		self.reg.a = r;
	}

	fn alu_cp(&mut self, b: u8) {
		let r = self.reg.a;
		self.alu_sub(b, false);
		self.reg.a = r;
	}

	fn alu_inc(&mut self, a: u8) -> u8 {
		let r = a + 1;
		self.reg.flag(Z, r == 0);
		self.reg.flag(H, (a & 0x0F) + 1 > 0x0F);
		self.reg.flag(N, false);
		return r
	}

	fn alu_dec(&mut self, a: u8) -> u8 {
		let r = a - 1;
		self.reg.flag(Z, r == 0);
		self.reg.flag(H, (a & 0x0F) == 0);
		self.reg.flag(N, true);
		return r
	}

	fn alu_add16(&mut self, b: u16) {
		let a = self.reg.hl();
		let r = a + b;
		self.reg.flag(H, (a & 0x07FF) + (b & 0x07FF) > 0x07FF);
		self.reg.flag(N, false);
		self.reg.flag(C, a > 0xFFFF - b);
		self.reg.sethl(r);
	}

	fn alu_add16imm(&mut self, a: u16) -> u16 {
		let b = self.fetchbyte() as i8 as i16 as u16;
		self.reg.flag(N, false);
		self.reg.flag(Z, false);
		self.reg.flag(H, (a & 0x000F) + (b & 0x000F) > 0x000F);
		self.reg.flag(C, (a & 0x00FF) + (b & 0x00FF) > 0x00FF);
		return a + b
	}

	fn alu_swap(&mut self, a: u8) -> u8 {
		self.reg.flag(Z, a == 0);
		self.reg.flag(C, false);
		self.reg.flag(H, false);
		self.reg.flag(N, false);
		(a >> 4) | (a << 4)
	}

	fn alu_srflagupdate(&mut self, r: u8, c: bool) {
		self.reg.flag(H, false);
		self.reg.flag(N, false);
		self.reg.flag(Z, r == 0);
		self.reg.flag(C, c);
	}

	fn alu_rlc(&mut self, a: u8) -> u8 {
		let c = a & 0x80 == 0x80;
		let r = (a << 1) | (if c { 1 } else { 0 });
		self.alu_srflagupdate(r, c);
		return r
	}

	fn alu_rl(&mut self, a: u8) -> u8 {
		let c = a & 0x80 == 0x80;
		let r = (a << 1) | (if self.reg.getflag(C) { 1 } else { 0 });
		self.alu_srflagupdate(r, c);
		return r
	}

	fn alu_rrc(&mut self, a: u8) -> u8 {
		let c = a & 0x01 == 0x01;
		let r = (a >> 1) | (if c { 0x80 } else { 0 });
		self.alu_srflagupdate(r, c);
		return r
	}

	fn alu_rr(&mut self, a: u8) -> u8 {
		let c = a & 0x01 == 0x01;
		let r = (a >> 1) | (if self.reg.getflag(C) { 0x80 } else { 0 });
		self.alu_srflagupdate(r, c);
		return r
	}

	fn alu_sla(&mut self, a: u8) -> u8 {
		let c = a & 0x80 == 0x80;
		let r = a << 1;
		self.alu_srflagupdate(r, c);
		return r
	}

	fn alu_sra(&mut self, a: u8) -> u8 {
		let c = a & 0x01 == 0x01;
		let r = (a >> 1) | (a & 0x80);
		self.alu_srflagupdate(r, c);
		return r
	}

	fn alu_srl(&mut self, a: u8) -> u8 {
		let c = a & 0x01 == 0x01;
		let r = a >> 1;
		self.alu_srflagupdate(r, c);
		return r
	}

	fn alu_bit(&mut self, a: u8, b: u8) {
		let r = a & (1 << (b as u32)) == 0;
		self.reg.flag(N, false);
		self.reg.flag(H, true);
		self.reg.flag(Z, r);
	}

	fn alu_daa(&mut self) {
		let mut a = self.reg.a;
		let mut adjust = if self.reg.getflag(C) { 0x60 } else { 0x00 };
		if self.reg.getflag(H) { adjust |= 0x06; };
		if !self.reg.getflag(N) {
			if a & 0x0F > 0x09 { adjust |= 0x06; };
			if a > 0x99 { adjust |= 0x60; };
			a += adjust;
		} else {
			a -= adjust;
		}

		self.reg.flag(C, adjust >= 0x60);
		self.reg.flag(H, false);
		self.reg.flag(Z, a == 0);
		self.reg.a = a;
	}

	fn cpu_jr(&mut self) {
		let n = self.fetchbyte() as i8;
		self.reg.pc = ((self.reg.pc as u32 as i32) + (n as i32)) as u16;
	}
}

#[cfg(test)]
mod test
{
	use super::CPU;
	use std::sync::{RwLock, Arc};

	static CPUINSTRS: &'static str = "roms/cpu_instrs.gb";

	#[test]
	fn cpu_instrs()
	{
		let sum_classic0 = Arc::new(RwLock::new(0));
		let sum_classic1 = sum_classic0.clone();
		let sum_color0 = Arc::new(RwLock::new(0));
		let sum_color1 = sum_color0.clone();

		let (tx, rx) = ::std::sync::mpsc::channel();
		let (mut r, mut w) = (::std::io::ChanReader::new(rx), ::std::io::ChanWriter::new(tx));

		let classic_t = ::std::thread::Thread::scoped(move||
		{
			let serial = |&mut: v| { let _ = w.write(&[v]); 0 };
			let mut c = match CPU::new(CPUINSTRS, Some(Box::new(serial) as ::serial::SerialCallback))
			{
				None => { panic!("Could not instantiate Classic CPU"); },
				Some(cpu) => cpu,
			};
			let mut ticks = 0;
			while ticks < 63802933
			{
				ticks += c.do_cycle();
			}
			let mut s = sum_classic1.write().unwrap();
			for i in range(0, c.mmu.gpu.data.len())
			{
				*s += (c.mmu.gpu.data[i] as u32) * (i as u32);
			}
		});

		let color_t = ::std::thread::Thread::scoped(move||
		{
			let mut c = match CPU::new_cgb(CPUINSTRS, None)
			{
				None => { panic!("Could not instantiate Color CPU"); },
				Some(cpu) => cpu,
			};
			let mut ticks = 0;
			while ticks < 63802933
			{
				ticks += c.do_cycle();
			}
			let mut s = sum_color1.write().unwrap();
			for i in range(0, c.mmu.gpu.data.len())
			{
				*s += (c.mmu.gpu.data[i] as u32) * (i as u32);
			}
		});

		classic_t.join().ok().unwrap();
		color_t.join().ok().unwrap();

		assert!(r.read_to_end().unwrap().as_slice() == b"cpu_instrs\n\n01:ok  02:ok  03:ok  04:ok  05:ok  06:ok  07:ok  08:ok  09:ok  10:ok  11:ok  \n\nPassed all tests\n",
			"cpu_instrs did not output the expected result to serial");
		assert!(*sum_classic0.read().unwrap() == 3112234583, "cpu_instrs was not graphically correct on Classic mode");
		assert!(*sum_color0.read().unwrap() == 479666872, "cpu_instrs was not graphically correct in Color mode");
	}
}
