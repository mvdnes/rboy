#![crate_name = "rboy"]
#![license    = "MIT" ]
#![feature(phase)]

#[phase(plugin, link)] extern crate log;
                       extern crate native;
                       extern crate getopts;
                       extern crate sdl;
                       extern crate rboy;

use rboy::device::Device;
use std::time::Duration;
use std::sync::{Arc,RWLock};
use std::comm::{Sender,Receiver,Disconnected,Empty};
use std::task::TaskBuilder;
use native::NativeTaskBuilder;

static SCALE: uint = 2;
static EXITCODE_INCORRECTOPTIONS: int = 1;
static EXITCODE_CPULOADFAILS: int = 2;

#[cfg(not(test))]
fn set_exit_status(exitcode: int)
{
    std::os::set_exit_status(exitcode);
}

fn main() {
	let args = std::os::args();
	let opts = &[ getopts::optflag("s", "serial", "Output serial to stdout"), getopts::optflag("c", "classic", "Force Classic mode") ];
	let matches = match getopts::getopts(args.tail(), opts) {
		Ok(m) => { m }
		Err(f) => { println!("{}", f); return }
	};

	let filename = if !matches.free.is_empty() {
		matches.free[0].clone()
	} else {
		let mut info_start = args[0].clone();
		info_start.push_str(" <filename>");
		println!("{}", getopts::usage(info_start.as_slice(), opts));
		set_exit_status(EXITCODE_INCORRECTOPTIONS);
		return;
	};

	sdl::init(&[sdl::InitFlag::Video]);
	sdl::wm::set_caption("RBoy - A gameboy in Rust", "rboy");
	let screen = match sdl::video::set_video_mode(160*SCALE as int, 144*SCALE as int, 32, &[sdl::video::HWSurface], &[sdl::video::DoubleBuf]) {
		Ok(screen) => screen,
		Err(err) => panic!("failed to open screen: {}", err),
	};

	let (sdl_tx, cpu_rx) = std::comm::channel();
	let (cpu_tx, sdl_rx) = std::comm::channel();
	let rawscreen = [0x00u8,.. 160*144*3];
	let arc = Arc::new(RWLock::new(rawscreen));
	let arc2 = arc.clone();

	TaskBuilder::new().native().spawn(proc() cpuloop(&cpu_tx, &cpu_rx, arc2, filename.as_slice(), &matches));

	let mut timer = std::io::timer::Timer::new().unwrap();
	let periodic = timer.periodic(Duration::milliseconds(8));

	'main : loop {
		periodic.recv();
		match sdl_rx.try_recv() {
			Err(Disconnected) => { break 'main },
			Ok(_) => recalculate_screen(&screen, &arc),
			Err(Empty) => {},
		}
		'event : loop {
			match sdl::event::poll_event() {
				sdl::event::Event::Quit => break 'main,
				sdl::event::Event::None => break 'event,
				sdl::event::Event::Key(sdl::event::Key::Escape, _, _, _)
					=> break 'main,
				sdl::event::Event::Key(sdl::event::Key::LShift, true, _, _)
					=> sdl_tx.send(GBEvent::SpeedUp),
				sdl::event::Event::Key(sdl::event::Key::LShift, false, _, _)
					=> sdl_tx.send(GBEvent::SlowDown),
				sdl::event::Event::Key(sdlkey, true, _, _) => {
					match sdl_to_keypad(sdlkey) {
						Some(key) => sdl_tx.send(GBEvent::KeyDown(key)),
						None => {},
					}
				},
				sdl::event::Event::Key(sdlkey, false, _, _) => {
					match sdl_to_keypad(sdlkey) {
						Some(key) => sdl_tx.send(GBEvent::KeyUp(key)),
						None => {},
					}
				},
				_ => {}
			}
		}
	}
}

fn sdl_to_keypad(key: sdl::event::Key) -> Option<rboy::KeypadKey> {
	match key {
		sdl::event::Key::Z => Some(rboy::KeypadKey::A),
		sdl::event::Key::X => Some(rboy::KeypadKey::B),
		sdl::event::Key::Up => Some(rboy::KeypadKey::Up),
		sdl::event::Key::Down => Some(rboy::KeypadKey::Down),
		sdl::event::Key::Left => Some(rboy::KeypadKey::Left),
		sdl::event::Key::Right => Some(rboy::KeypadKey::Right),
		sdl::event::Key::Space => Some(rboy::KeypadKey::Select),
		sdl::event::Key::Return => Some(rboy::KeypadKey::Start),
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
	KeyUp(rboy::KeypadKey),
	KeyDown(rboy::KeypadKey),
	SpeedUp,
	SlowDown,
}

fn cpuloop(cpu_tx: &Sender<uint>, cpu_rx: &Receiver<GBEvent>, arc: Arc<RWLock<[u8,.. 160*144*3]>>, filename: &str, matches: &getopts::Matches) {
	let opt_c = match matches.opt_present("classic") {
		true => Device::new(filename),
		false => Device::new_cgb(filename),
	};
	let mut c = match opt_c
	{
		Some(cpu) => { cpu },
		None => { error!("Could not get a valid gameboy"); set_exit_status(EXITCODE_CPULOADFAILS); return; },
	};
	c.set_stdout(matches.opt_present("serial"));

	let mut timer = std::io::timer::Timer::new().unwrap();
	let periodic = timer.periodic(Duration::milliseconds(8));
	let mut limit_speed = true;

	let waitticks = (4194.304f32 * 4.0) as uint;

	let mut ticks = 0;
	'cpuloop: loop {
		while ticks < waitticks {
			ticks += c.do_cycle();
			if c.check_and_reset_gpu_updated() {
				let mut data = arc.write();
				let gpu_data = c.get_gpu_data();
				for i in range(0, data.len()) { data[i] = gpu_data[i]; }
				if cpu_tx.send_opt(0).is_err() { break 'cpuloop };
			}
		}
		ticks -= waitticks;
		if limit_speed { periodic.recv(); }

		match cpu_rx.try_recv() {
			Ok(event) => match event {
				GBEvent::KeyUp(key) => c.keyup(key),
				GBEvent::KeyDown(key) => c.keydown(key),
				GBEvent::SpeedUp => limit_speed = false,
				GBEvent::SlowDown => limit_speed = true,
			},
			Err(Empty) => {},
			Err(Disconnected) => { break },
		};
	}
}
