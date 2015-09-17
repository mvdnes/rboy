#![crate_name = "rboy"]

extern crate clap;
extern crate sdl2;
extern crate rboy;

use rboy::device::Device;
use std::sync::{Arc,Mutex};
use std::sync::mpsc::{Sender,Receiver};
use std::sync::mpsc::TryRecvError::{Disconnected,Empty};
use std::error::Error;

const EXITCODE_SUCCESS : i32 = 0;
const EXITCODE_CPULOADFAILS : i32 = 2;

fn main() {
    let exit_status = real_main();
    if exit_status != EXITCODE_SUCCESS {
        std::process::exit(exit_status);
    }
}

fn real_main() -> i32 {
    let matches = clap::App::new("rboy")
        .version("0.1")
        .author("Mathijs van de Nes")
        .about("A Gameboy Colour emulator written in Rust")
        .arg(clap::Arg::with_name("filename")
             .help("Sets the ROM file to load")
             .required(true))
        .arg(clap::Arg::with_name("serial")
             .help("Prints the data from the serial port to stdout")
             .short("s")
             .long("serial"))
        .arg(clap::Arg::with_name("classic")
             .help("Forces the emulator to run in classic Gameboy mode")
             .short("c")
             .long("classic"))
        .arg(clap::Arg::with_name("scale")
             .help("Sets the scale of the interface. Default: 2")
             .short("x")
             .long("scale")
             .validator(|s|
                 match s.parse::<u32>() {
                     Err(e) => Err(format!("Could not parse scale: {}", e.description())),
                     Ok(s) if s < 1 => Err("Scale must be at least 1".to_owned()),
                     Ok(s) if s > 8 => Err("Scale may be at most 8".to_owned()),
                     Ok(..) => Ok(()),
                 })
             .takes_value(true))
        .get_matches();

    let opt_serial = matches.is_present("serial");
    let opt_classic = matches.is_present("classic");
    let filename = matches.value_of("filename").unwrap();
    let scale = matches.value_of("scale").unwrap_or("2").parse::<u32>().unwrap();

    let cpu = construct_cpu(filename, opt_classic, opt_serial);
    if cpu.is_none() { return EXITCODE_CPULOADFAILS; }
    let cpu = cpu.unwrap();

    let sdl_context = sdl2::init().unwrap();
    let sdl_video = sdl_context.video().ok().expect("Could not open video");
    let window = match sdl2::video::WindowBuilder::new(
            &sdl_video,
            "RBoy - A gameboy in Rust",
            160 * scale,
            144 * scale)
        .resizable()
        .build()
    {
        Ok(window) => window,
        Err(err) => panic!("failed to open window: {}", err),
    };
    let mut renderer = match sdl2::render::RendererBuilder::new(window).software().build() {
        Ok(screen) => screen,
        Err(err) => panic!("failed to open screen: {}", err),
    };
    let mut texture = renderer.create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGB24, (160, 144)).unwrap();

    let (sdl_tx, cpu_rx) = std::sync::mpsc::channel();
    let (cpu_tx, sdl_rx) = std::sync::mpsc::channel();
    let rawscreen = ::std::iter::repeat(0u8).take(160*144*3).collect();
    let arc = Arc::new(Mutex::new(rawscreen));
    let arc2 = arc.clone();

    let cpuloop_thread = std::thread::spawn(move|| cpuloop(&cpu_tx, &cpu_rx, arc2, cpu));

    let mut event_queue = sdl_context.event_pump().unwrap();
    'main : loop {
        let mut refreshed = false;
        'rx : loop {
            match sdl_rx.try_recv() {
                Err(Disconnected) => break 'main,
                Ok(_) => { if !refreshed { recalculate_screen(&mut renderer, &mut texture, &arc); refreshed = true } },
                Err(Empty) => break 'rx,
            }
        }
        for event in event_queue.wait_timeout_iter(2) {
            match event {
                sdl2::event::Event::Quit { .. } => break 'main,
                sdl2::event::Event::KeyDown { keycode: Some(sdl2::keyboard::Keycode::Escape), .. }
                    => break 'main,
                sdl2::event::Event::KeyDown { keycode: Some(sdl2::keyboard::Keycode::Num1), .. }
                    => renderer.window_mut().unwrap().set_size(160, 144),
                sdl2::event::Event::KeyDown { keycode: Some(sdl2::keyboard::Keycode::R), .. }
                    => renderer.window_mut().unwrap().set_size(160*scale, 144*scale),
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

    EXITCODE_SUCCESS
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

fn recalculate_screen(renderer: &mut sdl2::render::Renderer, texture: &mut sdl2::render::Texture, arc: &Arc<Mutex<Vec<u8>>>) {
    texture.update(None, &*arc.lock().unwrap().clone(), 160*3).unwrap();

    renderer.clear();
    renderer.copy(&texture, None, None);
    renderer.present();
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

fn construct_cpu(filename: &str, classic_mode: bool, output_serial: bool) -> Option<Device> {
    let opt_c = match classic_mode {
        true => Device::new(filename),
        false => Device::new_cgb(filename),
    };
    let mut c = match opt_c
    {
        Ok(cpu) => { cpu },
        Err(message) => { warn(message); return None; },
    };
    c.set_stdout(output_serial);
    Some(c)
}

fn cpuloop(cpu_tx: &Sender<()>, cpu_rx: &Receiver<GBEvent>, arc: Arc<Mutex<Vec<u8>>>, cpu: Device) {
    let mut c = cpu;
    let periodic = timer_periodic(8);
    let mut limit_speed = true;

    let waitticks = (4194304f64 / 1000.0 * 8.0) as u32;

    let mut ticks = 0;
    'cpuloop: loop {
        while ticks < waitticks {
            ticks += c.do_cycle();
            if c.check_and_reset_gpu_updated() {
                *arc.lock().unwrap() = c.get_gpu_data().to_vec();
                if cpu_tx.send(()).is_err() { break 'cpuloop };
            }
        }
        ticks -= waitticks;
        if limit_speed { let _ = periodic.recv(); }

        'rx : loop {
            match cpu_rx.try_recv() {
                Ok(event) => match event {
                    GBEvent::KeyUp(key) => c.keyup(key),
                    GBEvent::KeyDown(key) => c.keydown(key),
                    GBEvent::SpeedUp => limit_speed = false,
                    GBEvent::SlowDown => limit_speed = true,
                },
                Err(Empty) => break 'rx,
                Err(Disconnected) => break 'cpuloop,
            }
        }
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
