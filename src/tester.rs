#[crate_id = "tester"];

extern mod extra;
extern mod sdl;

use cpu::CPU;
use extra::getopts;
use extra::comm::DuplexStream;

mod register;
mod mmu;
mod cpu;
mod serial;
mod timer;
mod keypad;

fn main() {
	let args: ~[~str] = std::os::args();
	let program = args[0].clone() + " <filename>";

	let opts = ~[ getopts::groups::optflag("s", "serial", "Output serial to stdout") ];
	let matches = match getopts::groups::getopts(args.tail(), opts) {
		Ok(m) => { m }
		Err(f) => { println!("{}", f.to_err_msg()); return }
	};

	let filename: ~str = if !matches.free.is_empty() {
		matches.free[0].clone()
	} else {
		println!("{}", getopts::groups::usage(program, opts));
		return;
	};

	sdl::init([sdl::InitVideo]);
	sdl::wm::set_caption("RBoy - A gameboy in Rust", "rboy");
	let _screen = match sdl::video::set_video_mode(160, 144, 32, [sdl::video::HWSurface], [sdl::video::DoubleBuf]) {
		Ok(screen) => screen,
		Err(err) => fail!("failed to open screen: {}", err),
	};

	let (sdlstream, cpustream) = DuplexStream::new();

	spawn(proc() cpuloop(&cpustream, filename, &matches));

	'main : loop {
		'event : loop {
			match sdl::event::poll_event() {
				sdl::event::QuitEvent => break 'main,
				sdl::event::NoEvent => break 'event,
				sdl::event::KeyEvent(k, _, _, _)
					if k == sdl::event::EscapeKey
						=> break 'main,
				sdl::event::KeyEvent(k, _, _, _)
					if k == sdl::event::SpaceKey
						=>  { sdlstream.send(2); println!("{:u}", sdlstream.recv()); },
				_ => {}
			}
		}
	}
	sdlstream.send(1);
}

fn cpuloop(channel: &DuplexStream<uint, uint>, filename: ~str, matches: &getopts::Matches) {
	let mut c = CPU::new();
	c.mmu.loadrom(filename);
	c.mmu.serial.enabled = matches.opt_present("serial");

	loop {
		let ticks = c.cycle();
		match channel.try_recv() {
			Some(n) if n == 1 => { break; },
			Some(n) if n == 2 => { channel.send(ticks); },
			_ => {},
		};
	}
}
