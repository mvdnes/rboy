#![crate_name = "rboy"]

extern crate clap;
extern crate cpal;
extern crate glium;
extern crate rboy;

use glium::DisplayBuild;
use rboy::device::Device;
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::thread;
use std::error::Error;

const EXITCODE_SUCCESS : i32 = 0;
const EXITCODE_CPULOADFAILS : i32 = 2;

#[derive(Default)]
struct RenderOptions {
    pub linear_interpolation: bool,
}

enum GBEvent {
    KeyUp(rboy::KeypadKey),
    KeyDown(rboy::KeypadKey),
    SpeedUp,
    SpeedDown,
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
        .arg(clap::Arg::with_name("audio")
             .help("Enables audio")
             .short("a")
             .long("audio"))
        .get_matches();

    let opt_serial = matches.is_present("serial");
    let opt_classic = matches.is_present("classic");
    let opt_audio = matches.is_present("audio");
    let filename = matches.value_of("filename").unwrap();
    let scale = matches.value_of("scale").unwrap_or("2").parse::<u32>().unwrap();

    let cpu = construct_cpu(filename, opt_classic, opt_serial);
    if cpu.is_none() { return EXITCODE_CPULOADFAILS; }
    let mut cpu = cpu.unwrap();
    if opt_audio {
        let player = CpalPlayer::get();
        match player {
            Some(v) => cpu.enable_audio(Box::new(v) as Box<rboy::AudioPlayer>),
            None => { warn("Could not open audio device"); return EXITCODE_CPULOADFAILS; },
        }
    }
    let romname = cpu.romname();

    let (sender1, receiver1) = mpsc::channel();
    let (sender2, receiver2) = mpsc::sync_channel(1);

    let display = glium::glutin::WindowBuilder::new()
        .with_dimensions(rboy::SCREEN_W as u32 * scale, rboy::SCREEN_H as u32 * scale)
        .with_title("RBoy - ".to_owned() + &romname)
        .build_glium()
        .unwrap();

    let mut texture = glium::texture::texture2d::Texture2d::empty_with_format(
            &display,
            glium::texture::UncompressedFloatFormat::U8U8U8,
            glium::texture::MipmapsOption::NoMipmap,
            rboy::SCREEN_W as u32,
            rboy::SCREEN_H as u32)
        .unwrap();

    let mut renderoptions = <RenderOptions as Default>::default();

    let cputhread = thread::spawn(move|| run_cpu(cpu, sender2, receiver1));

    'main : loop {
        for ev in display.poll_events() {
            use glium::glutin::Event;
            use glium::glutin::ElementState::{Pressed, Released};
            use glium::glutin::VirtualKeyCode;

            match ev {
                Event::Closed
                    => break 'main,
                Event::KeyboardInput(Pressed, _, Some(VirtualKeyCode::Escape))
                    => break 'main,
                Event::KeyboardInput(Pressed, _, Some(VirtualKeyCode::Key1))
                    => display.get_window().unwrap().set_inner_size(rboy::SCREEN_W as u32, rboy::SCREEN_H as u32),
                Event::KeyboardInput(Pressed, _, Some(VirtualKeyCode::R))
                    => display.get_window().unwrap().set_inner_size(rboy::SCREEN_W as u32 * scale, rboy::SCREEN_H as u32 * scale),
                Event::KeyboardInput(Pressed, _, Some(VirtualKeyCode::LShift))
                    => { let _ = sender1.send(GBEvent::SpeedUp); },
                Event::KeyboardInput(Released, _, Some(VirtualKeyCode::LShift))
                    => { let _ = sender1.send(GBEvent::SpeedDown); },
                Event::KeyboardInput(Pressed, _, Some(VirtualKeyCode::T))
                    => { renderoptions.linear_interpolation = !renderoptions.linear_interpolation; }
                Event::KeyboardInput(Pressed, _, Some(glutinkey)) => {
                    if let Some(key) = glutin_to_keypad(glutinkey) {
                        let _ = sender1.send(GBEvent::KeyDown(key));
                    }
                },
                Event::KeyboardInput(Released, _, Some(glutinkey)) => {
                    if let Some(key) = glutin_to_keypad(glutinkey) {
                        let _ = sender1.send(GBEvent::KeyUp(key));
                    }
                },
                _ => (),
            }
        }

        match receiver2.try_recv() {
            Ok(data) => recalculate_screen(&display, &mut texture, &*data, &renderoptions),
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => break 'main,
        }
    }

    drop(sender1);
    let _ = cputhread.join();

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
                      datavec: &[u8],
                      renderoptions: &RenderOptions)
{
    use glium::Surface;

    let interpolation_type = if renderoptions.linear_interpolation {
        glium::uniforms::MagnifySamplerFilter::Linear
    }
    else {
        glium::uniforms::MagnifySamplerFilter::Nearest
    };

    let rawimage2d = glium::texture::RawImage2d {
        data: std::borrow::Cow::Borrowed(datavec),
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

fn run_cpu(mut cpu: Device, sender: SyncSender<Vec<u8>>, receiver: Receiver<GBEvent>) {
    let periodic = timer_periodic(16);
    let mut limit_speed = true;

    let waitticks = (4194304f64 / 1000.0 * 16.0).round() as u32;
    let mut ticks = 0;

    'outer: loop {
        while ticks < waitticks {
            ticks += cpu.do_cycle();
            if cpu.check_and_reset_gpu_updated() {
                let data = cpu.get_gpu_data().to_vec();
                if let Err(TrySendError::Disconnected(..)) = sender.try_send(data) {
                    break 'outer;
                }
            }
        }

        ticks -= waitticks;

        'recv: loop {
            match receiver.try_recv() {
                Ok(event) => {
                    match event {
                        GBEvent::KeyUp(key) => cpu.keyup(key),
                        GBEvent::KeyDown(key) => cpu.keydown(key),
                        GBEvent::SpeedUp => limit_speed = false,
                        GBEvent::SpeedDown => { limit_speed = true; cpu.sync_audio(); }
                    }
                },
                Err(TryRecvError::Empty) => break 'recv,
                Err(TryRecvError::Disconnected) => break 'outer,
            }
        }

        if limit_speed { let _ = periodic.recv(); }
    }
}

fn timer_periodic(ms: u64) -> Receiver<()> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(ms));
            if tx.send(()).is_err() {
                break;
            }
        }
    });
    rx
}

struct CpalPlayer {
    voice: cpal::Voice,
}

impl CpalPlayer {
    fn get() -> Option<CpalPlayer> {
        if cpal::get_endpoints_list().count() == 0 { return None; }

        let endpoint = match cpal::get_default_endpoint() {
            Some(e) => e,
            None => return None,
        };

        let format = match endpoint.get_supported_formats_list().ok().and_then(|mut v| v.next()) {
            Some(f) => f,
            None => return None,
        };

        cpal::Voice::new(&endpoint, &format).ok().map(|v| CpalPlayer { voice: v })
    }
}

impl rboy::AudioPlayer for CpalPlayer {
    fn play(&mut self, buf_left: &[f32], buf_right: &[f32]) {
        debug_assert!(buf_left.len() == buf_right.len());

        let left_idx = self.voice.format().channels.iter().position(|c| *c == cpal::ChannelPosition::FrontLeft);
        let right_idx = self.voice.format().channels.iter().position(|c| *c == cpal::ChannelPosition::FrontRight);

        let channel_count = self.voice.format().channels.len();

        let count = buf_left.len();
        let mut done = 0;
        let mut lastdone = count;

        while lastdone != done && done < count {
            lastdone = done;
            let buf_left_next = &buf_left[done..];
            let buf_right_next = &buf_right[done..];
            match self.voice.append_data(count - done) {
                cpal::UnknownTypeBuffer::U16(mut buffer) => {
                    for (i, sample) in buffer.chunks_mut(channel_count).enumerate() {
                        if let Some(idx) = left_idx {
                            sample[idx] = (buf_left_next[i] * (std::i16::MAX as f32) + (std::i16::MAX as f32)) as u16;
                        }
                        if let Some(idx) = right_idx {
                            sample[idx] = (buf_right_next[i] * (std::i16::MAX as f32) + (std::i16::MAX as f32)) as u16;
                        }
                        done += 1;
                    }
                }
                cpal::UnknownTypeBuffer::I16(mut buffer) => {
                    for (i, sample) in buffer.chunks_mut(channel_count).enumerate() {
                        if let Some(idx) = left_idx {
                            sample[idx] = (buf_left_next[i] * std::i16::MAX as f32) as i16;
                        }
                        if let Some(idx) = right_idx {
                            sample[idx] = (buf_right_next[i] * std::i16::MAX as f32) as i16;
                        }
                        done += 1;
                    }
                }
                cpal::UnknownTypeBuffer::F32(mut buffer) => {
                    for (i, sample) in buffer.chunks_mut(channel_count).enumerate() {
                        if let Some(idx) = left_idx {
                            sample[idx] = buf_left_next[i];
                        }
                        if let Some(idx) = right_idx {
                            sample[idx] = buf_right_next[i];
                        }
                        done += 1;
                    }
                }
            }
        }
        self.voice.play();
    }

    fn samples_rate(&self) -> u32 {
        self.voice.format().samples_rate.0
    }

    fn underflowed(&self) -> bool {
        self.voice.underflowed()
    }
}
