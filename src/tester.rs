use mmu::MMU;
use cpu::CPU;

mod mmu;
mod cpu;

fn main() {
	let mut m = MMU::new();
	let mut c = CPU::new();

	loop {
		let t = c.cycle(&mut m);
		m.cycle(t);
	}
}
