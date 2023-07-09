#![crate_name = "rboy"]

use rboy::device::Device;
use std::io::{self, Read};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread;
use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
use glium::glutin::platform::run_return::EventLoopExtRunReturn;

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

#[cfg(target_os = "windows")]
fn create_window_builder(romname: &str)-> glium::glutin::window::WindowBuilder{
    use glium::glutin::platform::windows::WindowBuilderExtWindows;
    return glium::glutin::window::WindowBuilder::new()
        .with_drag_and_drop(false)
        .with_inner_size(glium::glutin::dpi::LogicalSize::<u32>::from((
            rboy::SCREEN_W as u32,
            rboy::SCREEN_H as u32,
        )))
        .with_title("RBoy - ".to_owned() + romname);
}

#[cfg(not(target_os = "windows"))]
fn create_window_builder(romname: &str)-> glium::glutin::window::WindowBuilder {
    return glium::glutin::window::WindowBuilder::new()
            .with_inner_size(glium::glutin::dpi::LogicalSize::<u32>::from((
                rboy::SCREEN_W as u32,
                rboy::SCREEN_H as u32,
            )))
            .with_title("RBoy - ".to_owned() + romname);
}

#[derive(Debug)]
struct ArgParseError {
    message: String,
}

impl ArgParseError {
    fn new<T: Into<String>>(message: T) -> Self {
        ArgParseError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ArgParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ArgParseError {}

fn parse_scale_var(arg: &str) -> Result<u32, ArgParseError> {
    match arg.parse::<u32>() {
        Err(e) => Err(ArgParseError::new(format!("Could not parse scale: {}", e))),
        Ok(s) if s < 1 => Err(ArgParseError::new("Scale must be at least 1")),
        Ok(s) if s > 8 => Err(ArgParseError::new("Scale may be at most 8")),
        Ok(s) => Ok(s),
    }
}

fn main() {
    let exit_status = real_main();
    if exit_status != EXITCODE_SUCCESS {
        std::process::exit(exit_status);
    }
}

fn real_main() -> i32 {
    let matches = clap::Command::new("rboy")
        .version("0.1")
        .author("Mathijs van de Nes")
        .about("A Gameboy Colour emulator written in Rust")
        .arg(clap::Arg::new("filename")
             .help("Sets the ROM file to load")
             .required(true))
        .arg(clap::Arg::new("serial")
             .help("Prints the data from the serial port to stdout")
             .short('s')
             .long("serial")
             .action(clap::ArgAction::SetTrue))
        .arg(clap::Arg::new("printer")
             .help("Emulates a gameboy printer")
             .short('p')
             .long("printer")
             .action(clap::ArgAction::SetTrue))
        .arg(clap::Arg::new("classic")
             .help("Forces the emulator to run in classic Gameboy mode")
             .short('c')
             .long("classic")
             .action(clap::ArgAction::SetTrue))
        .arg(clap::Arg::new("scale")
             .help("Sets the scale of the interface. Default: 2")
             .short('x')
             .long("scale")
             .value_parser(parse_scale_var))
        .arg(clap::Arg::new("audio")
             .help("Enables audio")
             .short('a')
             .long("audio")
             .action(clap::ArgAction::SetTrue))
        .arg(clap::Arg::new("skip-checksum")
             .help("Skips verification of the cartridge checksum")
             .long("skip-checksum")
             .action(clap::ArgAction::SetTrue))
        .arg(clap::Arg::new("test-mode")
             .help("Starts the emulator in a special test mode")
             .long("test-mode")
             .action(clap::ArgAction::SetTrue))
        .get_matches();

    let test_mode = matches.get_one::<bool>("test-mode").copied().unwrap();
    let opt_serial = matches.get_one::<bool>("serial").copied().unwrap();
    let opt_printer = matches.get_one::<bool>("printer").copied().unwrap();
    let opt_classic = matches.get_one::<bool>("classic").copied().unwrap();
    let opt_audio = matches.get_one::<bool>("audio").copied().unwrap();
    let opt_skip_checksum = matches.get_one::<bool>("skip-checksum").copied().unwrap();
    let filename = matches.get_one::<String>("filename").unwrap();
    let scale = matches.get_one::<u32>("scale").copied().unwrap_or(2);

    if test_mode {
        return run_test_mode(filename, opt_classic, opt_skip_checksum);
    }

    let cpu = construct_cpu(filename, opt_classic, opt_serial, opt_printer, opt_skip_checksum);
    if cpu.is_none() { return EXITCODE_CPULOADFAILS; }
    let mut cpu = cpu.unwrap();

    let mut cpal_audio_stream = None;
    if opt_audio {
        let player = CpalPlayer::get();
        match player {
            Some((v, s)) => {
                cpu.enable_audio(Box::new(v) as Box<dyn rboy::AudioPlayer>);
                cpal_audio_stream = Some(s);
            },
            None => {
                warn("Could not open audio device");
                return EXITCODE_CPULOADFAILS;
            },
        }
    }
    let romname = cpu.romname();

    let (sender1, receiver1) = mpsc::channel();
    let (sender2, receiver2) = mpsc::sync_channel(1);

    let mut eventloop = glium::glutin::event_loop::EventLoop::new();
    let window_builder = create_window_builder(&romname);
    let context_builder = glium::glutin::ContextBuilder::new();
    let display = glium::backend::glutin::Display::new(window_builder, context_builder, &eventloop).unwrap();
    set_window_size(display.gl_window().window(), scale);

    let mut texture = glium::texture::texture2d::Texture2d::empty_with_format(
            &display,
            glium::texture::UncompressedFloatFormat::U8U8U8,
            glium::texture::MipmapsOption::NoMipmap,
            rboy::SCREEN_W as u32,
            rboy::SCREEN_H as u32)
        .unwrap();

    let mut renderoptions = <RenderOptions as Default>::default();

    let cputhread = thread::spawn(move|| run_cpu(cpu, sender2, receiver1));

    eventloop.run_return(move |ev, _evtarget, controlflow| {
        use glium::glutin::event::{Event, WindowEvent, KeyboardInput};
        use glium::glutin::event::ElementState::{Pressed, Released};
        use glium::glutin::event::VirtualKeyCode;

        let mut stop = false;
        match ev {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested
                    => stop = true,
                WindowEvent::KeyboardInput { input, .. } => match input {
                    KeyboardInput { state: Pressed, virtual_keycode: Some(VirtualKeyCode::Escape), .. }
                        => stop = true,
                    KeyboardInput { state: Pressed, virtual_keycode: Some(VirtualKeyCode::Key1), .. }
                        => set_window_size(display.gl_window().window(), 1),
                    KeyboardInput { state: Pressed, virtual_keycode: Some(VirtualKeyCode::R), .. }
                        => set_window_size(display.gl_window().window(), scale),
                    KeyboardInput { state: Pressed, virtual_keycode: Some(VirtualKeyCode::LShift), .. }
                        => { let _ = sender1.send(GBEvent::SpeedUp); },
                    KeyboardInput { state: Released, virtual_keycode: Some(VirtualKeyCode::LShift), .. }
                        => { let _ = sender1.send(GBEvent::SpeedDown); },
                    KeyboardInput { state: Pressed, virtual_keycode: Some(VirtualKeyCode::T), .. }
                        => { renderoptions.linear_interpolation = !renderoptions.linear_interpolation; }
                    KeyboardInput { state: Pressed, virtual_keycode: Some(glutinkey), .. } => {
                        if let Some(key) = glutin_to_keypad(glutinkey) {
                            let _ = sender1.send(GBEvent::KeyDown(key));
                        }
                    },
                    KeyboardInput { state: Released, virtual_keycode: Some(glutinkey), .. } => {
                        if let Some(key) = glutin_to_keypad(glutinkey) {
                            let _ = sender1.send(GBEvent::KeyUp(key));
                        }
                    },
                    _ => (),
                },
                _ => (),
            },
            Event::MainEventsCleared => {
                match receiver2.recv() {
                    Ok(data) => recalculate_screen(&display, &mut texture, &*data, &renderoptions),
                    Err(..) => stop = true, // Remote end has hung-up
                }
            },
            _ => (),
        }
        if stop == true {
            *controlflow = glium::glutin::event_loop::ControlFlow::Exit;
        }
    });

    drop(cpal_audio_stream);
    let _ = cputhread.join();

    EXITCODE_SUCCESS
}

fn glutin_to_keypad(key: glium::glutin::event::VirtualKeyCode) -> Option<rboy::KeypadKey> {
    use glium::glutin::event::VirtualKeyCode;
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

fn recalculate_screen(display: &glium::Display,
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

fn warn(message: &str) {
    eprintln!("{}", message);
}

fn construct_cpu(filename: &str, classic_mode: bool, output_serial: bool, output_printer: bool, skip_checksum: bool) -> Option<Box<Device>> {
    let opt_c = match classic_mode {
        true => Device::new(filename, skip_checksum),
        false => Device::new_cgb(filename, skip_checksum),
    };
    let mut c = match opt_c
    {
        Ok(cpu) => { cpu },
        Err(message) => { warn(message); return None; },
    };

    if output_printer {
        c.attach_printer();
    }
    else {
        c.set_stdout(output_serial);
    }

    Some(Box::new(c))
}

fn run_cpu(mut cpu: Box<Device>, sender: SyncSender<Vec<u8>>, receiver: Receiver<GBEvent>) {
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

fn set_window_size(window: &glium::glutin::window::Window, scale: u32) {
    use glium::glutin::dpi::{LogicalSize, PhysicalSize};

    let dpi = window.scale_factor();

    let physical_size = PhysicalSize::<u32>::from((rboy::SCREEN_W as u32 * scale, rboy::SCREEN_H as u32 * scale));
    let logical_size = LogicalSize::<u32>::from_physical(physical_size, dpi);

    window.set_inner_size(logical_size);
}

struct CpalPlayer {
    buffer: Arc<Mutex<Vec<(f32, f32)>>>,
    sample_rate: u32,
}

impl CpalPlayer {
    fn get() -> Option<(CpalPlayer, cpal::Stream)> {
        let device = match cpal::default_host().default_output_device() {
            Some(e) => e,
            None => return None,
        };

        // We want a config with:
        // chanels = 2
        // SampleFormat F32
        // Rate at around 44100

        let wanted_samplerate = cpal::SampleRate(44100);
        let supported_configs = match device.supported_output_configs() {
            Ok(e) => e,
            Err(_) => return None,
        };
        let mut supported_config = None;
        for f in supported_configs {
            if f.channels() == 2 && f.sample_format() == cpal::SampleFormat::F32 {
                if f.min_sample_rate() <= wanted_samplerate && wanted_samplerate <= f.max_sample_rate() {
                    supported_config = Some(f.with_sample_rate(wanted_samplerate));
                }
                else {
                    supported_config = Some(f.with_max_sample_rate());
                }
                break;
            }
        }
        if supported_config.is_none() {
            return None;
        }

        let selected_config = supported_config.unwrap();

        let sample_format = selected_config.sample_format();
        let config : cpal::StreamConfig = selected_config.into();

        let err_fn = |err| eprintln!("An error occurred on the output audio stream: {}", err);

        let shared_buffer = Arc::new(Mutex::new(Vec::new()));
        let stream_buffer = shared_buffer.clone();

        let player = CpalPlayer {
            buffer: shared_buffer,
            sample_rate: config.sample_rate.0,
        };

        let stream = match sample_format {
            cpal::SampleFormat::F32 => device.build_output_stream(&config, move|data: &mut [f32], _callback_info: &cpal::OutputCallbackInfo| cpal_thread(data, &stream_buffer), err_fn),
            cpal::SampleFormat::U16 => device.build_output_stream(&config, move|data: &mut [u16], _callback_info: &cpal::OutputCallbackInfo| cpal_thread(data, &stream_buffer), err_fn),
            cpal::SampleFormat::I16 => device.build_output_stream(&config, move|data: &mut [i16], _callback_info: &cpal::OutputCallbackInfo| cpal_thread(data, &stream_buffer), err_fn),
        }.unwrap();

        stream.play().unwrap();

        Some((player, stream))
    }
}

fn cpal_thread<T: cpal::Sample>(outbuffer: &mut[T], audio_buffer: &Arc<Mutex<Vec<(f32, f32)>>>) {
    let mut inbuffer = audio_buffer.lock().unwrap();
    let outlen =  ::std::cmp::min(outbuffer.len() / 2, inbuffer.len());
    for (i, (in_l, in_r)) in inbuffer.drain(..outlen).enumerate() {
        outbuffer[i*2] = cpal::Sample::from(&in_l);
        outbuffer[i*2+1] = cpal::Sample::from(&in_r);
    }
}

impl rboy::AudioPlayer for CpalPlayer {
    fn play(&mut self, buf_left: &[f32], buf_right: &[f32]) {
        debug_assert!(buf_left.len() == buf_right.len());

        let mut buffer = self.buffer.lock().unwrap();

        for (l, r) in buf_left.iter().zip(buf_right) {
            if buffer.len() > self.sample_rate as usize {
                // Do not fill the buffer with more than 1 second of data
                // This speeds up the resync after the turning on and off the speed limiter
                return
            }
            buffer.push((*l, *r));
        }
    }

    fn samples_rate(&self) -> u32 {
        self.sample_rate
    }

    fn underflowed(&self) -> bool {
        (*self.buffer.lock().unwrap()).len() == 0
    }
}

struct NullAudioPlayer {}

impl rboy::AudioPlayer for NullAudioPlayer {
    fn play(&mut self, _buf_left: &[f32], _buf_right: &[f32]) {
        // Do nothing
    }

    fn samples_rate(&self) -> u32 {
        44100
    }

    fn underflowed(&self) -> bool {
        false
    }
}

fn run_test_mode(filename: &str, classic_mode: bool, skip_checksum: bool) -> i32 {
    let opt_cpu = match classic_mode {
        true => Device::new(filename, skip_checksum),
        false => Device::new_cgb(filename, skip_checksum),
    };
    let mut cpu = match opt_cpu {
        Err(errmsg) => { warn(errmsg); return EXITCODE_CPULOADFAILS; },
        Ok(cpu) => cpu,
    };

    cpu.set_stdout(true);
    cpu.enable_audio(Box::new(NullAudioPlayer {}));

    // from masonforest, https://stackoverflow.com/a/55201400 (CC BY-SA 4.0)
    let stdin_channel = spawn_stdin_channel();
    loop {
        match stdin_channel.try_recv() {
            Ok(stdin_byte) => {
                match stdin_byte {
                    b'q' => break,
                    b's' => {
                        let data = cpu.get_gpu_data().to_vec();
                        print_screenshot(data);
                    },
                    v => {
                        eprintln!("MSG:Unknown stdinvalue {}", v);
                    },
                }
            },
            Err(TryRecvError::Empty) => {},
            Err(TryRecvError::Disconnected) => break,
        }
        for _ in 0..1000 {
            cpu.do_cycle();
        }
    }
    EXITCODE_SUCCESS
}

fn spawn_stdin_channel() -> Receiver<u8> {
    let (tx, rx) = mpsc::channel::<u8>();
    thread::spawn(move || loop {
        let mut buffer = [0];
        match io::stdin().read(&mut buffer) {
            Ok(1) => tx.send(buffer[0]).unwrap(),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {},
            _ => break,
        };
    });
    rx
}

fn print_screenshot(data: Vec<u8>) {
    eprint!("SCREENSHOT:");
    for b in data {
        eprint!("{:02x}", b);
    };
    eprintln!();
}
