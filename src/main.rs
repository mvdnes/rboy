#![crate_id = "rboy"]
#![license = "MIT"]

#![feature(phase)]
#[phase(plugin, link)] extern crate log;

extern crate native;
extern crate getopts;
extern crate time;
extern crate sdl;
#[cfg(test)]
extern crate test;

use cpu::CPU;
use std::sync::{Arc,RWLock};
use std::comm::{DuplexStream,Disconnected,Empty};
use std::task::TaskBuilder;
use native::NativeTaskBuilder;

mod register;
mod mbc;
mod mmu;
mod cpu;
mod serial;
mod timer;
mod keypad;
mod gpu;
mod sound;
mod gbmode;
mod util;

static SCALE: uint = 2;

#[cfg(not(test))]
#[start]
fn start(argc: int, argv: **u8) -> int { native::start(argc, argv, main) }

fn main() {
	let args = std::os::args();
	let opts = [ getopts::optflag("s", "serial", "Output serial to stdout"), getopts::optflag("c", "classic", "Force Classic mode") ];
	let matches = match getopts::getopts(args.tail(), opts) {
		Ok(m) => { m }
		Err(f) => { println!("{}", f); return }
	};

	let filename = if !matches.free.is_empty() {
		matches.free.get(0).clone()
	} else {
		println!("{}", getopts::usage(args.get(0).clone().append(" <filename>").as_slice(), opts));
		return;
	};

	sdl::init([sdl::InitVideo]);
	sdl::wm::set_caption("RBoy - A gameboy in Rust", "rboy");
	let screen = match sdl::video::set_video_mode(160*SCALE as int, 144*SCALE as int, 32, [sdl::video::HWSurface], [sdl::video::DoubleBuf]) {
		Ok(screen) => screen,
		Err(err) => fail!("failed to open screen: {}", err),
	};

	let (sdlstream, cpustream) = std::comm::duplex();
	let rawscreen = [0x00u8,.. 160*144*3];
	let arc = Arc::new(RWLock::new(rawscreen));
	let arc2 = arc.clone();

	TaskBuilder::new().native().spawn(proc() cpuloop(&cpustream, arc2, filename.as_slice(), &matches));

	let mut timer = std::io::timer::Timer::new().unwrap();
	let periodic = timer.periodic(8);

	'main : loop {
		periodic.recv();
		match sdlstream.try_recv() {
			Err(Disconnected) => { break 'main },
			Ok(_) => recalculate_screen(&screen, &arc),
			Err(Empty) => {},
		}
		'event : loop {
			match sdl::event::poll_event() {
				sdl::event::QuitEvent => break 'main,
				sdl::event::NoEvent => break 'event,
				sdl::event::KeyEvent(sdl::event::EscapeKey, _, _, _)
					=> break 'main,
				sdl::event::KeyEvent(sdl::event::LShiftKey, true, _, _)
					=> sdlstream.send(SpeedUp),
				sdl::event::KeyEvent(sdl::event::LShiftKey, false, _, _)
					=> sdlstream.send(SlowDown),
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

fn recalculate_screen(screen: &sdl::video::Surface, arc: &Arc<RWLock<[u8,.. 160*144*3]>>) {
	let data = arc.read();
	for y in range(0u, 144) {
		for x in range(0u, 160) {
			screen.fill_rect(
				Some(sdl::Rect { x: (x*SCALE) as i16, y: (y*SCALE) as i16, w: SCALE as u16, h: SCALE as u16 }),
				sdl::video::RGB(data[y*160*3 + x*3 + 0],
				                data[y*160*3 + x*3 + 1],
				                data[y*160*3 + x*3 + 2])
			);
		}
	}
	screen.flip();
}

enum GBEvent {
	KeyUp(keypad::KeypadKey),
	KeyDown(keypad::KeypadKey),
	SpeedUp,
	SlowDown,
}

fn cpuloop(channel: &DuplexStream<uint, GBEvent>, arc: Arc<RWLock<[u8,.. 160*144*3]>>, filename: &str, matches: &getopts::Matches) {
	let opt_c = match matches.opt_present("classic") {
		true => CPU::new(filename),
		false => CPU::new_cgb(filename),
	};
	let mut c = match opt_c
	{
		Some(cpu) => { cpu },
		None => { error!("Could not get a valid gameboy"); return; },
	};
	c.mmu.serial.tostdout = matches.opt_present("serial");

	let mut timer = std::io::timer::Timer::new().unwrap();
	let mut periodic = timer.periodic(8);

	let waitticks = (4194.304 * 4.0) as uint;

	let mut ticks = 0;
	'cpuloop: loop {
		while ticks < waitticks {
			ticks += c.cycle();
			if c.mmu.gpu.updated {
				c.mmu.gpu.updated = false;
				let mut data = arc.write();
				for i in range(0, data.len()) { data[i] = c.mmu.gpu.data[i]; }
				if channel.send_opt(0).is_err() { break 'cpuloop };
			}
		}
		ticks -= waitticks;
		periodic.recv();

		match channel.try_recv() {
			Ok(event) => match event {
				KeyUp(key) => c.mmu.keypad.keyup(key),
				KeyDown(key) => c.mmu.keypad.keydown(key),
				SpeedUp => periodic = timer.periodic(1),
				SlowDown => periodic = timer.periodic(8),
			},
			Err(Empty) => {},
			Err(Disconnected) => { break },
		};
	}
}
