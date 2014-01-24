use mmu::MMU;
use cpu::CPU;

mod mmu;
mod cpu;

fn main() {
	let mut m = MMU::new();
	let mut c = CPU::new();

	c.cycle(&mut m);
}
