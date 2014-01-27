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
				sdl::event::KeyEvent(sdl::event::EscapeKey, _, _, _)
					=> break 'main,
				sdl::event::KeyEvent(sdlkey, true, _, _) => {
					match sdl_to_keypad(sdlkey) {
						Some(key) => sdlstream.send(KeyDown(key)),
						None => {},
					}
				},
				sdl::event::KeyEvent(sdlkey, false, _, _) => {
					match sdl_to_keypad(sdlkey) {
						Some(key) => sdlstream.send(KeyUp(key)),
						None => {},
					}
				},
				_ => {}
			}
		}
	}
	sdlstream.send(Poweroff);
}

fn sdl_to_keypad(key: sdl::event::Key) -> Option<keypad::KeypadKey> {
	match key {
		sdl::event::ZKey => Some(keypad::A),
		sdl::event::XKey => Some(keypad::B),
		sdl::event::UpKey => Some(keypad::Up),
		sdl::event::DownKey => Some(keypad::Down),
		sdl::event::LeftKey => Some(keypad::Left),
		sdl::event::RightKey => Some(keypad::Right),
		sdl::event::SpaceKey => Some(keypad::Select),
		sdl::event::ReturnKey => Some(keypad::Start),
		_ => None,
	}
}

enum GBEvent {
	KeyUp(keypad::KeypadKey),
	KeyDown(keypad::KeypadKey),
	Poweroff,
}

fn cpuloop(channel: &DuplexStream<uint, GBEvent>, filename: ~str, matches: &getopts::Matches) {
	let mut c = CPU::new();
	c.mmu.loadrom(filename);
	c.mmu.serial.enabled = matches.opt_present("serial");

	loop {
		c.cycle();
		match channel.try_recv() {
			None => {},
			Some(Poweroff) => { break; },
			Some(KeyUp(key)) => c.mmu.keypad.keyup(key),
			Some(KeyDown(key)) => c.mmu.keypad.keydown(key),
		};
	}
}
