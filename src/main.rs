#![crate_name = "rboy"]

extern crate clap;
extern crate glium;
extern crate rboy;

use glium::DisplayBuild;
use rboy::device::Device;
use std::sync::{Arc,Mutex};
use std::sync::mpsc::{Sender,Receiver};
use std::sync::mpsc::TryRecvError::{Disconnected,Empty};
use std::error::Error;

const EXITCODE_SUCCESS : i32 = 0;
const EXITCODE_CPULOADFAILS : i32 = 2;

#[derive(Default)]
struct RenderOptions {
    pub linear_interpolation: bool,
}

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

    let display = glium::glutin::WindowBuilder::new()
        .with_dimensions(rboy::SCREEN_W as u32 * scale, rboy::SCREEN_H as u32 * scale)
        .with_title("RBoy - A gameboy in Rust".to_owned())
        .build_glium()
        .unwrap();

    let mut texture = glium::texture::texture2d::Texture2d::empty_with_format(
            &display,
            glium::texture::UncompressedFloatFormat::U8U8U8,
            glium::texture::MipmapsOption::NoMipmap,
            rboy::SCREEN_W as u32,
            rboy::SCREEN_H as u32)
        .unwrap();

    let (sdl_tx, cpu_rx) = std::sync::mpsc::channel();
    let (cpu_tx, sdl_rx) = std::sync::mpsc::channel();
    let rawscreen = ::std::iter::repeat(0u8).take(rboy::SCREEN_W * rboy::SCREEN_H * 3).collect();
    let arc = Arc::new(Mutex::new(rawscreen));
    let arc2 = arc.clone();

    let cpuloop_thread = std::thread::spawn(move|| cpuloop(&cpu_tx, &cpu_rx, arc2, cpu));

    let mut renderoptions = Default::default();

    'main : loop {
        let mut refreshed = false;
        'rx : loop {
            match sdl_rx.try_recv() {
                Err(Disconnected) => break 'main,
                Ok(_) => { if !refreshed { recalculate_screen(&display, &mut texture, &arc, &renderoptions); refreshed = true } },
                Err(Empty) => break 'rx,
            }
        }
        for ev in display.poll_events() {
            use glium::glutin::Event;
            use glium::glutin::ElementState::{Pressed, Released};
            use glium::glutin::VirtualKeyCode;

            match ev {
                Event::Closed
                    => break 'main,
                Event::Resized(..)
                    => { if !refreshed { recalculate_screen(&display, &mut texture, &arc, &renderoptions); refreshed = true } },
                Event::KeyboardInput(Pressed, _, Some(VirtualKeyCode::Escape))
                    => break 'main,
                Event::KeyboardInput(Pressed, _, Some(VirtualKeyCode::Key1))
                    => display.get_window().unwrap().set_inner_size(rboy::SCREEN_W as u32, rboy::SCREEN_H as u32),
                Event::KeyboardInput(Pressed, _, Some(VirtualKeyCode::R))
                    => display.get_window().unwrap().set_inner_size(rboy::SCREEN_W as u32 * scale, rboy::SCREEN_H as u32 * scale),
                Event::KeyboardInput(Pressed, _, Some(VirtualKeyCode::LShift))
                    => { let _ = sdl_tx.send(GBEvent::SpeedUp); },
                Event::KeyboardInput(Released, _, Some(VirtualKeyCode::LShift))
                    => { let _ = sdl_tx.send(GBEvent::SlowDown); },
                Event::KeyboardInput(Pressed, _, Some(VirtualKeyCode::T))
                    => { renderoptions.linear_interpolation = !renderoptions.linear_interpolation; }
                Event::KeyboardInput(Pressed, _, Some(glutinkey)) => {
                    match glutin_to_keypad(glutinkey) {
                        Some(key) =>  { let _ = sdl_tx.send(GBEvent::KeyDown(key)); },
                        None => {},
                    }
                },
                Event::KeyboardInput(Released, _, Some(glutinkey)) => {
                    match glutin_to_keypad(glutinkey) {
                        Some(key) => { let _ = sdl_tx.send(GBEvent::KeyUp(key)); },
                        None => {},
                    }
                },
                _ => (),
            }
        }
    }

    drop(sdl_tx); // Disconnect such that the cpuloop will exit
    let _ = cpuloop_thread.join();

    EXITCODE_SUCCESS
}

fn glutin_to_keypad(key: glium::glutin::VirtualKeyCode) -> Option<rboy::KeypadKey> {
    use glium::glutin::VirtualKeyCode;
    match key {
        VirtualKeyCode::Z => Some(rboy::KeypadKey::A),
        VirtualKeyCode::X => Some(rboy::KeypadKey::B),
        VirtualKeyCode::Up => Some(rboy::KeypadKey::Up),
        VirtualKeyCode::Down => Some(rboy::KeypadKey::Down),
        VirtualKeyCode::Left => Some(rboy::KeypadKey::Left),
        VirtualKeyCode::Right => Some(rboy::KeypadKey::Right),
        VirtualKeyCode::Space => Some(rboy::KeypadKey::Select),
        VirtualKeyCode::Return => Some(rboy::KeypadKey::Start),
        _ => None,
    }
}

fn recalculate_screen(display: &glium::backend::glutin_backend::GlutinFacade,
                      texture: &mut glium::texture::texture2d::Texture2d,
                      arc: &Arc<Mutex<Vec<u8>>>,
                      renderoptions: &RenderOptions)
{
    use glium::Surface;

    let interpolation_type = if renderoptions.linear_interpolation {
        glium::uniforms::MagnifySamplerFilter::Linear
    }
    else {
        glium::uniforms::MagnifySamplerFilter::Nearest
    };

    {
        // Scope to release the Mutex as soon as possible
        let datavec = arc.lock().unwrap();
        let rawimage2d = glium::texture::RawImage2d {
            data: std::borrow::Cow::Borrowed(&**datavec),
            width: rboy::SCREEN_W as u32,
            height: rboy::SCREEN_H as u32,
            format: glium::texture::ClientFormat::U8U8U8,
        };
        texture.write(
            glium::Rect {
                left: 0,
                bottom: 0,
                width: rboy::SCREEN_W as u32,
                height: rboy::SCREEN_H as u32
            },
            rawimage2d);
    }

    // We use a custom BlitTarget to transform OpenGL coordinates to row-column coordinates
    let target = display.draw();
    let (target_w, target_h) = target.get_dimensions();
    texture.as_surface().blit_whole_color_to(
        &target,
        &glium::BlitTarget {
            left: 0,
            bottom: target_h,
            width: target_w as i32,
            height: -(target_h as i32)
        },
        interpolation_type);
    target.finish().unwrap();
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
                let mut data = arc.lock().unwrap();
                let gpudata = c.get_gpu_data();
                for i in 0..data.len() { data[i] = gpudata[i]; }
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
