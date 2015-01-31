#![crate_name = "rboy"]

#![feature(io, std_misc, core, collections, os)]

#[macro_use]
extern crate log;

extern crate getopts;

extern crate sdl2;

extern crate rboy;

use rboy::device::Device;
use std::time::Duration;
use std::sync::{Arc,RwLock};
use std::sync::mpsc::{Sender,Receiver};
use std::sync::mpsc::TryRecvError::{Disconnected,Empty};
use std::old_io::timer;

static SCALE: usize = 2;
static EXITCODE_INCORRECTOPTIONS: isize = 1;
static EXITCODE_CPULOADFAILS: isize = 2;

#[cfg(not(test))]
fn set_exit_status(exitcode: isize)
{
	std::os::set_exit_status(exitcode);
}

fn main() {
	let args = std::os::args();

	let mut opts = getopts::Options::new();
	opts.optflag("s", "serial", "Output serial to stdout");
	opts.optflag("c", "classic", "Force Classic mode");

	let matches = match opts.parse(args.tail()) {
		Ok(m) => { m }
		Err(f) => { println!("{}", f); return }
	};

	let filename = if !matches.free.is_empty() {
		matches.free[0].clone()
	} else {
		let mut info_start = args[0].clone();
		info_start.push_str(" <filename>");
		println!("{}", opts.usage(info_start.as_slice()));
		set_exit_status(EXITCODE_INCORRECTOPTIONS);
		return;
	};

	sdl2::init(sdl2::INIT_VIDEO);
	let window = match sdl2::video::Window::new("RBoy - A gameboy in Rust",
												sdl2::video::WindowPos::PosUndefined,
												sdl2::video::WindowPos::PosUndefined,
												160*SCALE as i32,
												144*SCALE as i32,
												sdl2::video::WindowFlags::empty()) {
		Ok(window) => window,
		Err(err) => panic!("failed to open window: {}", err),
	};
	let renderer = match sdl2::render::Renderer::from_window(window, sdl2::render::RenderDriverIndex::Auto, sdl2::render::ACCELERATED) {
		Ok(screen) => screen,
		Err(err) => panic!("failed to open screen: {}", err),
	};

	let (sdl_tx, cpu_rx) = std::sync::mpsc::channel();
	let (cpu_tx, sdl_rx) = std::sync::mpsc::channel();
	let rawscreen = ::std::iter::repeat(0u8).take(160*144*3).collect();
	let arc = Arc::new(RwLock::new(rawscreen));
	let arc2 = arc.clone();

	let cpuloop_thread = std::thread::Thread::scoped(move|| cpuloop(&cpu_tx, &cpu_rx, arc2, filename.as_slice(), &matches));

	let mut timer = timer::Timer::new().unwrap();
	let periodic = timer.periodic(Duration::milliseconds(8));

	'main : loop {
		let _ = periodic.recv();
		match sdl_rx.try_recv() {
			Err(Disconnected) => { break 'main },
			Ok(_) => recalculate_screen(&renderer, &arc),
			Err(Empty) => {},
		}
		'event : loop {
			match sdl2::event::poll_event() {
				sdl2::event::Event::Quit { .. } => break 'main,
				sdl2::event::Event::None => break 'event,
				sdl2::event::Event::KeyDown { keycode: sdl2::keycode::KeyCode::Escape, .. }
					=> break 'main,
				sdl2::event::Event::KeyDown { keycode: sdl2::keycode::KeyCode::LShift, .. }
					=> { let _ = sdl_tx.send(GBEvent::SpeedUp); },
				sdl2::event::Event::KeyUp { keycode: sdl2::keycode::KeyCode::LShift, .. }
					=> { let _ = sdl_tx.send(GBEvent::SlowDown); },
				sdl2::event::Event::KeyDown { keycode: sdlkey, .. } => {
					match sdl_to_keypad(sdlkey) {
						Some(key) =>  { let _ = sdl_tx.send(GBEvent::KeyDown(key)); },
						None => {},
					}
				},
				sdl2::event::Event::KeyUp { keycode: sdlkey, .. } => {
					match sdl_to_keypad(sdlkey) {
						Some(key) => { let _ = sdl_tx.send(GBEvent::KeyUp(key)); },
						None => {},
					}
				},
				_ => {}
			}
		}
	}

	drop(sdl_tx); // Disconnect such that the cpuloop will exit
	let _ = cpuloop_thread.join();
}

fn sdl_to_keypad(key: sdl2::keycode::KeyCode) -> Option<rboy::KeypadKey> {
	match key {
		sdl2::keycode::KeyCode::Z => Some(rboy::KeypadKey::A),
		sdl2::keycode::KeyCode::X => Some(rboy::KeypadKey::B),
		sdl2::keycode::KeyCode::Up => Some(rboy::KeypadKey::Up),
		sdl2::keycode::KeyCode::Down => Some(rboy::KeypadKey::Down),
		sdl2::keycode::KeyCode::Left => Some(rboy::KeypadKey::Left),
		sdl2::keycode::KeyCode::Right => Some(rboy::KeypadKey::Right),
		sdl2::keycode::KeyCode::Space => Some(rboy::KeypadKey::Select),
		sdl2::keycode::KeyCode::Return => Some(rboy::KeypadKey::Start),
		_ => None,
	}
}

fn recalculate_screen(screen: &sdl2::render::Renderer, arc: &Arc<RwLock<Vec<u8>>>) {
	let mut drawer = screen.drawer();
	drawer.set_draw_color(sdl2::pixels::Color::RGB(0xFF, 0xFF, 0xFF));
	drawer.clear();

	let data =  arc.read().unwrap().clone();
	for y in range(0, 144) {
		for x in range(0, 160) {
			drawer.set_draw_color(sdl2::pixels::Color::RGB(data[y*160*3 + x*3 + 0],
															data[y*160*3 + x*3 + 1],
															data[y*160*3 + x*3 + 2]));
			drawer.draw_rect(&sdl2::rect::Rect::new((x*SCALE) as i32, (y*SCALE) as i32, SCALE as i32, SCALE as i32));
		}
	}
	drawer.present();
}

enum GBEvent {
	KeyUp(rboy::KeypadKey),
	KeyDown(rboy::KeypadKey),
	SpeedUp,
	SlowDown,
}

fn cpuloop(cpu_tx: &Sender<u32>, cpu_rx: &Receiver<GBEvent>, arc: Arc<RwLock<Vec<u8>>>, filename: &str, matches: &getopts::Matches) {
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

	let mut timer = timer::Timer::new().unwrap();
	let periodic = timer.periodic(Duration::milliseconds(8));
	let mut limit_speed = true;

	let waitticks = (4194.304f32 * 4.0) as u32;

	let mut ticks = 0;
	'cpuloop: loop {
		while ticks < waitticks {
			ticks += c.do_cycle();
			if c.check_and_reset_gpu_updated() {
				let mut data = arc.write().unwrap();
				let gpu_data = c.get_gpu_data();
				for i in range(0, data.len()) { data[i] = gpu_data[i]; }
				if cpu_tx.send(0).is_err() { break 'cpuloop };
			}
		}
		ticks -= waitticks;
		if limit_speed { let _ = periodic.recv(); }

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
