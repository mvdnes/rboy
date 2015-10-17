use blip_buf::BlipBuf;
use cpal;
use std;

macro_rules! try_opt {
     ( $expr:expr ) => {
         match $expr {
             Some(v) => v,
             None => return None,
         }
     }
}

const WAVE_PATTERN : [[u8; 8]; 4] = [[0,0,0,0,1,0,0,0],[0,0,0,0,1,1,0,0],[0,0,1,1,1,1,0,0],[1,1,1,1,0,0,1,1]];

struct VolumeEnvelope {
    period : u32,
    goes_up : bool,
    delay : u32,
    initial_volume : u32,
    volume : u32,
}

impl VolumeEnvelope {
    fn new() -> VolumeEnvelope {
        VolumeEnvelope {
            period: 0,
            goes_up: false,
            delay: 0,
            initial_volume: 0,
            volume: 0,
        }
    }

    fn wb(&mut self, a: u16, v: u8) {
        match a {
            0xFF12 | 0xFF17 | 0xFF21 => {
                self.period = v & 0x7;
                self.goes_up = v & 0x8 == 0x8;
                self.initial_volume = v >> 4;
                self.volume = self.initial_volume;
            },
            0xFF14 | 0xFF19 | 0xFF23 if v & 0x80 == 0x80 => {
                self.delay = self.period;
                self.volume = self.initial_volume;
                // enabled = true
            },
        }
    }

    fn step(&mut self) {
        if self.delay > 1 {
            self.delay -= 1;
        }
        else if self.delay == 1 {
            self.delay = self.period;
            if self.goes_up && self.volume < 15 {
                self.volume += 1;
            }
            else if !self.goes_up && self.volume > 0 {
                self.volume -= 1;
            }
        }
    }
}

struct SquareChannel {
    enabled : bool,
    duty : u8,
    phase : u8,
    length: u8,
    length_enabled : bool,
    has_sweep : bool,
    frequency: u32,
    period: u32,
    last_amp: i32,
    delay: u32,
    volume_envelope: VolumeEnvelope,
    blip: BlipBuf,
}

impl SquareChannel {
    fn new(with_sweep: bool, blip: BlipBuf) -> SquareChannel {
        SquareChannel {
            enabled: false,
            duty: 0,
            phase: 0,
            length: 0,
            length_enabled: false,
            has_sweep: with_sweep,
            frequency: 0,
            period: 0,
            last_amp: 0,
            delay: 0,
            volume_envelope: VolumeEnvelope::new(),
            blip: blip,
        }
    }

    fn run(&mut self) {
    }
}

pub struct Sound {
    on: bool,
    registerdata: [u8; 0x17],
    time: u32,
    voice: cpal::Voice,
}

impl Sound {
    pub fn new() -> Option<Sound> {
        let voice = match get_channel() {
            Some(v) => v,
            None => {
                println!("Could not open audio device");
                return None;
            },
        };

        Sound {
            on: false,
            registerdata: [0, 0x17],
            time: 0,
            voice: voice,
        }
    }

    fn create_blipbuf(voice: &cpal::Voice) -> BlipBuf {
        let mut blipbuf = BlipBuf::new(voice.format().samples_rate.0);
        blipbuf.set_rates((1 << 22) as f64, voice.format().samples_rate.0 as f64);
        blipbuf
    }

    pub fn rb(&self, a: u16) -> u8 {
        // run
        match a {
            // 0xFF16 => self.channel2_duty << 6,
            // 0xFF17 => self.channel2_vol << 4 | if self.channel2_volup { 8 } else { 0 } | self.channel2_volsweep,
            // 0xFF18 => 0,
            // 0xFF19 => if self.channel2_started { 1 << 6 } else { 0 },
            // 0xFF1A => if self.channel3_on { 0x80 } else { 0 },
            // 0xFF1B => self.channel3_len,
            // 0xFF1C => self.channel3_vol << 5,
            // 0xFF1D => 0,
            // 0xFF1E => 0,
            // 0xFF26 => (if self.on { 0x80 } else { 0 })
            //     | (if self.channel2_started { 2 } else { 0 })
            //     | (if self.channel3_started { 4 } else { 0 }),
            // 0xFF30 ... 0xFF3F => {
            //     let wave_a = a as usize - 0xFF30;
            //     self.waveram[wave_a * 2] << 4 | self.waveram[wave_a * 2 + 1]
            // },
            0xFF10 ... 0xFF25 => self.registerdata[a - 0xFF10],
            0xFF26 => {
                self.registerdata[a - 0xFF10] & 0xF0
                // add information about other channels
            }
            //0xFF30 ... 0xFF3F =>
            _ => 0,
        }
    }

    pub fn wb(&mut self, a: u16, v: u8) {
        if a != 0xFF26 && !self.on { return; }
        // run
        match a {
            0xFF10 ... 0xFF25 => self.registerdata[a - 0xFF10],
            0xFF26 => self.on = v & 0x80 == 0x80,
            // 0xFF30 ... 0xFF3F => {
            //     let wave_a = a as usize - 0xFF30;
            //     self.waveram[wave_a * 2] = v >> 4;
            //     self.waveram[wave_a * 2 + 1] = v & 0xF;
            // },
            _ => (),
        }
    }

    #[inline]
    fn blip(&mut self) -> &mut BlipBuf {
        self.blip.as_mut().unwrap()
    }

    #[inline]
    fn channel(&mut self) -> &mut cpal::Voice {
        self.voice.as_mut().unwrap()
    }

    pub fn do_cycle(&mut self, cycles: u32)
    {
        if !self.on { return; }
        self.time += cycles;
        self.hz256 += cycles;

        let trigger256 = if self.hz256 >= (1 << 22) / 256 {
            self.hz256 -= (1 << 22) / 256;

            /*
            let time = self.time;
            let newblip = if self.blipval == 10000 {
                -10000
            }
            else {
                10000
            };
            self.blipval += newblip;
            self.blip().add_delta(time, newblip);
            */
            true
        }
        else {
            false
        };

        if self.channel2_started {
            let rfreq = 32 * (2048 - self.channel2_freq);
            self.channel2_freq_div += cycles;
            if self.channel2_uselen && trigger256 {
                if self.channel2_len == 0 {
                    self.channel2_len = 63;
                }
                else {
                    self.channel2_len -= 1;
                }
                if self.channel2_len == 0 {
                    self.channel2_started = false;
                }
            }

            if self.channel2_freq_div >= rfreq {
                self.channel2_freq_div -= rfreq;
                self.channel2_duty_cnt = (self.channel2_duty_cnt + 1) % 8;

                self.channel2_volcnt = (self.channel2_volcnt + 1) % 8;
                if self.channel2_volcnt == 0 && self.channel2_volsweep != 0 {
                    self.channel2_volsweep -= 1;
                    if self.channel2_volup && self.channel2_vol != 0xF {
                        self.channel2_vol += 1;
                    }
                    else if self.channel2_vol != 0 {
                        self.channel2_vol -= 1;
                    }
                }

                let sample = WAVE_PATTERN[self.channel2_duty as usize][self.channel2_duty_cnt as usize];

                if self.blip.is_some() {
                    let newblip = (sample as f64 * self.channel2_vol as f64 * (1.0/15.0) * 10000.0).round() as i32 - self.blipval;
                    let time = self.time;
                    self.blip().add_delta(time, newblip);
                    self.blipval += newblip;
                }
            }
        }

        /*
        if self.channel3_started && self.channel3_on {
            let rfreq = 32 * (2048 - (self.channel3_freq as u32));
            self.channel3_freq_div += cycles;
            if self.channel3_uselen && trigger256 {
                self.channel3_len = self.channel3_len.wrapping_sub(1);
                if self.channel3_len == 0 {
                    self.channel3_started = false;
                }
            }
            if self.channel3_freq_div >= rfreq {
                self.channel3_freq_div -= rfreq;
                self.channel3_wave_idx = (self.channel3_wave_idx + 1) % 32;
                let sample = self.waveram[self.channel3_wave_idx];
                let volmul = match self.channel3_vol {
                    1 => 1.0,
                    2 => 0.5,
                    3 => 0.25,
                    _ => 0.0,
                };
                let newblip = ((sample as f64 / 7.5 - 1.0) * volmul * 10000.0) as i32 - self.blipval;
                let time = self.time;
                if self.blip.is_some() {
                    self.blip().add_delta(time, newblip);
                    self.blipval += newblip;
                }
            }
        }*/
/*        else if self.blipval != 0 && self.blip.is_some() {
            let time = self.time;
            let newblip = -self.blipval;
            self.blip().add_delta(time, newblip);
            self.blipval = 0;
        }*/
        if self.time >= (1 << 16) && self.blip.is_some() {
            self.blip().end_frame(1 << 16);
            self.time -= 1 << 16;
            self.play_blipbuf();
        }
    }

    fn play_blipbuf(&mut self) {
        let channels_len = self.channel().format().channels.len();

        while self.blip().samples_avail() > 0 {
            let buf = &mut [0; 2048];
            let count = self.blip().read_samples(buf, false);
            let blipbuf = &buf[..count];
            let mut done = 0;
            let mut lastdone = count;

            while lastdone != done && done < count {
                lastdone = done;
                let channelbuf = &blipbuf[done..];
                match self.channel().append_data(channelbuf.len()) {
                    cpal::UnknownTypeBuffer::U16(mut buffer) => {
                        for (sample, value) in buffer.chunks_mut(channels_len).zip(channelbuf) {
                            let value = *value as u16 + std::i16::MAX as u16;
                            for out in sample.iter_mut() { *out = value; }
                            done += 1;
                        }
                    }
                    cpal::UnknownTypeBuffer::I16(mut buffer) => {
                        for (sample, value) in buffer.chunks_mut(channels_len).zip(channelbuf) {
                            for out in sample.iter_mut() { *out = *value; }
                            done += 1;
                        }
                    }
                    cpal::UnknownTypeBuffer::F32(mut buffer) => {
                        for (sample, value) in buffer.chunks_mut(channels_len).zip(channelbuf) {
                            let value = *value as f32 / std::i16::MAX as f32;
                            for out in sample.iter_mut() { *out = value; }
                            done += 1;
                        }
                    }
                }
            }
            self.channel().play();
        }
    }
}

fn get_channel() -> Option<cpal::Voice> {
    if cpal::get_endpoints_list().count() == 0 { return None; }

    let endpoint = try_opt!(cpal::get_default_endpoint());
    let format = try_opt!(endpoint.get_supported_formats_list().ok().and_then(|mut v| v.next()));

    cpal::Voice::new(&endpoint, &format).ok()
}
