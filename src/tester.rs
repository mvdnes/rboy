#[crate_id = "tester"]

use mmu::MMU;
use cpu::CPU;
use std::str;

mod register;
mod mmu;
mod cpu;
mod serial;
mod timer;

fn main() {
	let args: ~[~str] = std::os::args();
	if args.len() < 2 { return };
	let mut m = MMU::new(args[1]);
	let mut c = CPU::new();

	loop {
		let t = c.cycle(&mut m);
		m.cycle(t);
	}
}
