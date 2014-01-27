#[crate_id = "tester"];

extern mod extra;

use cpu::CPU;
use extra::getopts;

mod register;
mod mmu;
mod cpu;
mod serial;
mod timer;

fn main() {
	let args: ~[~str] = std::os::args();
	let program = args[0].clone() + " <filename>";

	let opts = ~[ getopts::groups::optflag("s", "serial", "Output serial to stdout") ];
	let matches = match getopts::groups::getopts(args.tail(), opts) {
		Ok(m) => { m }
		Err(f) => { println!("{}", f.to_err_msg()); return }
	};

	let filename: &str = if !matches.free.is_empty() {
		matches.free[0].clone()
	} else {
		println!("{}", getopts::groups::usage(program, opts));
		return;
	};

	let mut c = CPU::new();
	c.mmu.loadrom(filename);
	c.mmu.serial.enabled = matches.opt_present("serial");

	loop {
		c.cycle();
	}
}
