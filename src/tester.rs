#[crate_id = "tester"];

extern mod extra;
extern mod sdl;

use cpu::CPU;
use extra::getopts;
use extra::comm::DuplexStream;
use extra::arc::RWArc;

mod register;
mod mmu;
mod cpu;
mod serial;
mod timer;
mod keypad;
mod gpu;

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
	let screen = match sdl::video::set_video_mode(160, 144, 32, [sdl::video::HWSurface], [sdl::video::DoubleBuf]) {
		Ok(screen) => screen,
		Err(err) => fail!("failed to open screen: {}", err),
	};

	let (sdlstream, cpustream) = DuplexStream::new();
	let rawscreen = ~[0xFFu8,.. 160*144*3];
	let arc = RWArc::new(rawscreen);
	let arc2 = arc.clone();
	spawn(proc() cpuloop(&cpustream, arc2, filename, &matches));

	'main : loop {
		match sdlstream.try_recv() {
			Some(_) => recalculate_screen(screen, &arc),
			None => {},
		}
		'event : loop {
			match sdl::event::poll_event() {
				sdl::event::QuitEvent => break 'main,
				sdl::event::NoEvent => break 'event,
				sdl::event::KeyEvent(sdl::event::EscapeKey, _, _, _)
					=> break 'main,
				sdl::event::KeyEvent(sdlkey, true, _, _) => {
					match sdl_to_keypad(sdlkey) {
						Some(key) => { screen.flip(); sdlstream.send(KeyDown(key)) },
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

fn recalculate_screen(screen: &sdl::video::Surface, arc: &RWArc<~[u8]>) {
	arc.read(|data| 
	for y in range(0, 144) {
		for x in range(0, 160) {
			screen.fill_rect(
				Some(sdl::Rect { x: x as i16, y: y as i16, w: 1, h: 1 }),
				sdl::video::RGB(data[y*160*3 + x*3 + 0],
				                data[y*160*3 + x*3 + 1],
				                data[y*160*3 + x*3 + 2])
			);
		}
	});
	screen.flip();
}

enum GBEvent {
	KeyUp(keypad::KeypadKey),
	KeyDown(keypad::KeypadKey),
	Poweroff,
}

fn cpuloop(channel: &DuplexStream<uint, GBEvent>, arc: RWArc<~[u8]>, filename: ~str, matches: &getopts::Matches) {
	let mut c = CPU::new();
	c.mmu.loadrom(filename);
	c.mmu.serial.enabled = matches.opt_present("serial");

	loop {
		c.cycle();

		if c.mmu.gpu.updated {
			c.mmu.gpu.updated = false;
			arc.write(|data|
				for i in range(0, c.mmu.gpu.data.len()) {
					data[i] = c.mmu.gpu.data[i];
				}
			);
			channel.send(0);
		}

		match channel.try_recv() {
			None => {},
			Some(Poweroff) => { break; },
			Some(KeyUp(key)) => c.mmu.keypad.keyup(key),
			Some(KeyDown(key)) => c.mmu.keypad.keydown(key),
		};
	}
}
