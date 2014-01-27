#[crate_id = "tester"];

use cpu::CPU;

mod register;
mod mmu;
mod cpu;
mod serial;
mod timer;

fn main() {
	let args: ~[~str] = std::os::args();
	if args.len() < 2 { return };

	let mut c = CPU::new();
	c.mmu.loadrom(args[1]);

	loop {
		c.cycle();
	}
}
