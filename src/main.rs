#![crate_name = "rboy"]

extern crate sdl2;
extern crate rboy;

use rboy::device::Device;
use std::sync::{Arc,RwLock};
use std::sync::mpsc::{Sender,Receiver};
use std::sync::mpsc::TryRecvError::{Disconnected,Empty};
use std::sync::atomic;
use std::sync::atomic::Ordering::Relaxed;

const SCALE : u32 = 2;
const EXITCODE_SUCCESS : isize = 0;
const EXITCODE_INCORRECTOPTIONS : isize = 1;
const EXITCODE_CPULOADFAILS : isize = 2;

static EXITCODE : atomic::AtomicIsize = atomic::ATOMIC_ISIZE_INIT;

fn main() {
    set_exit_status(EXITCODE_SUCCESS);
    real_main();
    std::process::exit(EXITCODE.load(Relaxed) as i32);
}

fn set_exit_status(status: isize) {
    EXITCODE.store(status, Relaxed);
}

fn print_usage() {
    println!("usage: rboy <filename> [-s|--serial] [-c|--classic]");
    set_exit_status(EXITCODE_INCORRECTOPTIONS);
}

fn real_main() {
    let args: Vec<_> = std::env::args().collect();

    match args.len() {
        0 ... 1 => { print_usage(); return }
        _ => {}
    };

    let mut opt_serial = false;
    let mut opt_classic = false;
    let mut file = None;
    for argument in &args[1..] {
        let arg = &argument[..];
        if arg == "-s" || arg == "--serial" { opt_serial = true; }
        else if arg == "-c" || arg == "--classic" { opt_classic = true; }
        else { file = Some(argument.clone()); }
    }

    let filename = if let Some(ref f) = file {
        f.clone()
    } else {
        print_usage();
        return;
    };

    let sdl_context = sdl2::init().unwrap();
    let sdl_video = sdl_context.video().unwrap();
    let window = match sdl2::video::WindowBuilder::new(&sdl_video,
                                                "RBoy - A gameboy in Rust",
                                                160*SCALE,
                                                144*SCALE).build() {
        Ok(window) => window,
        Err(err) => panic!("failed to open window: {}", err),
    };
    let mut renderer = match sdl2::render::RendererBuilder::new(window).accelerated().build() {
        Ok(screen) => screen,
        Err(err) => panic!("failed to open screen: {}", err),
    };

    let (sdl_tx, cpu_rx) = std::sync::mpsc::channel();
    let (cpu_tx, sdl_rx) = std::sync::mpsc::channel();
    let rawscreen = ::std::iter::repeat(0u8).take(160*144*3).collect();
    let arc = Arc::new(RwLock::new(rawscreen));
    let arc2 = arc.clone();

    let cpuloop_thread = std::thread::spawn(move|| cpuloop(&cpu_tx, &cpu_rx, arc2, &filename, opt_classic, opt_serial));

    let periodic = timer_periodic(8);

    let mut event_queue = sdl_context.event_pump().unwrap();
    'main : loop {
        let _ = periodic.recv();
        match sdl_rx.try_recv() {
            Err(Disconnected) => { break 'main },
            Ok(_) => recalculate_screen(&mut renderer, &arc),
            Err(Empty) => {},
        }
        for event in event_queue.poll_iter() {
            match event {
                sdl2::event::Event::Quit { .. } => break 'main,
                sdl2::event::Event::KeyDown { keycode: Some(sdl2::keyboard::Keycode::Escape), .. }
                    => break 'main,
                sdl2::event::Event::KeyDown { keycode: Some(sdl2::keyboard::Keycode::LShift), .. }
                    => { let _ = sdl_tx.send(GBEvent::SpeedUp); },
                sdl2::event::Event::KeyUp { keycode: Some(sdl2::keyboard::Keycode::LShift), .. }
                    => { let _ = sdl_tx.send(GBEvent::SlowDown); },
                sdl2::event::Event::KeyDown { keycode: Some(sdlkey), .. } => {
                    match sdl_to_keypad(sdlkey) {
                        Some(key) =>  { let _ = sdl_tx.send(GBEvent::KeyDown(key)); },
                        None => {},
                    }
                },
                sdl2::event::Event::KeyUp { keycode: Some(sdlkey), .. } => {
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

fn sdl_to_keypad(key: sdl2::keyboard::Keycode) -> Option<rboy::KeypadKey> {
    match key {
        sdl2::keyboard::Keycode::Z => Some(rboy::KeypadKey::A),
        sdl2::keyboard::Keycode::X => Some(rboy::KeypadKey::B),
        sdl2::keyboard::Keycode::Up => Some(rboy::KeypadKey::Up),
        sdl2::keyboard::Keycode::Down => Some(rboy::KeypadKey::Down),
        sdl2::keyboard::Keycode::Left => Some(rboy::KeypadKey::Left),
        sdl2::keyboard::Keycode::Right => Some(rboy::KeypadKey::Right),
        sdl2::keyboard::Keycode::Space => Some(rboy::KeypadKey::Select),
        sdl2::keyboard::Keycode::Return => Some(rboy::KeypadKey::Start),
        _ => None,
    }
}

fn recalculate_screen(screen: &mut sdl2::render::Renderer, arc: &Arc<RwLock<Vec<u8>>>) {
    screen.set_draw_color(sdl2::pixels::Color::RGB(0xFF, 0xFF, 0xFF));
    screen.clear();

    let data =  arc.read().unwrap().clone();
    for y in 0..144 {
        for x in 0..160 {
            screen.set_draw_color(sdl2::pixels::Color::RGB(data[y*160*3 + x*3 + 0],
                                                            data[y*160*3 + x*3 + 1],
                                                            data[y*160*3 + x*3 + 2]));
            screen.draw_rect(sdl2::rect::Rect::new(x as i32 * SCALE as i32, y as i32 * SCALE as i32, SCALE, SCALE).unwrap().unwrap());
        }
    }
    screen.present();
}

enum GBEvent {
    KeyUp(rboy::KeypadKey),
    KeyDown(rboy::KeypadKey),
    SpeedUp,
    SlowDown,
}

fn warn(message: &'static str) {
    use std::io::Write;
    let _ = write!(&mut std::io::stderr(), "{}\n", message);
}

fn cpuloop(cpu_tx: &Sender<u32>, cpu_rx: &Receiver<GBEvent>, arc: Arc<RwLock<Vec<u8>>>, filename: &str, need_c: bool, need_s: bool) {
    let opt_c = match need_c {
        true => Device::new(filename),
        false => Device::new_cgb(filename),
    };
    let mut c = match opt_c
    {
        Ok(cpu) => { cpu },
        Err(message) => { warn(message); set_exit_status(EXITCODE_CPULOADFAILS); return; },
    };
    c.set_stdout(need_s);

    let periodic = timer_periodic(8);
    let mut limit_speed = true;

    let waitticks = (4194.304f32 * 4.0) as u32;

    let mut ticks = 0;
    'cpuloop: loop {
        while ticks < waitticks {
            ticks += c.do_cycle();
            if c.check_and_reset_gpu_updated() {
                let mut data = arc.write().unwrap();
                let gpu_data = c.get_gpu_data();
                for i in 0..data.len() { data[i] = gpu_data[i]; }
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

fn timer_periodic(ms: u32) -> Receiver<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep_ms(ms);
            if tx.send(()).is_err() {
                break;
            }
        }
    });
    rx
}
