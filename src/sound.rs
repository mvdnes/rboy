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

pub struct Sound {
    waveram: [u8; 16],
    channel3_on: bool,
    channel3_len: u8,
    channel3_vol: u8,
    channel3_freq: u32,
    channel3_freq_div: u32,
    channel3_uselen: bool,
    channel3_started: bool,
    channel3_wave_idx: usize,
    voice: Option<cpal::Voice>,
    blip: Option<BlipBuf>,
    bliptime: u32,
    blipval: i32,
    hz256: u32,
}

impl Sound {
    pub fn new() -> Sound {
        let voice = get_channel();
        if voice.is_none() {
            println!("Could not open audio device");
        }

        let blipbuf = voice.as_ref()
            .map(|v| {
                let mut bb = BlipBuf::new(v.format().samples_rate.0 / 10);
                bb.set_rates((1 << 22) as f64, v.format().samples_rate.0 as f64);
                bb
            });

        Sound {
            waveram: [0; 16],
            channel3_on: false,
            channel3_len: 0,
            channel3_vol: 0,
            channel3_freq: 0,
            channel3_uselen: false,
            channel3_started: false,
            channel3_freq_div: 0,
            channel3_wave_idx: 0,
            voice: voice,
            blip: blipbuf,
            bliptime: 0,
            blipval: 0,
            hz256: 0,
        }
    }

   pub fn rb(&self, a: u16) -> u8 {
        match a {
            0xFF1A => if self.channel3_on { 0x80 } else { 0 },
            0xFF1B => self.channel3_len,
            0xFF1C => self.channel3_vol << 5,
            0xFF1D => 0,
            0xFF1E => 0,
            0xFF30 ... 0xFF3F => self.waveram[a as usize - 0xFF30],
            _ => 0,
        }
    }

    pub fn wb(&mut self, a: u16, v: u8) {
        match a {
            0xFF1A => if v & 0x80 == 0x80 { self.channel3_on = true; } else { self.channel3_started = false; },
            0xFF1B => self.channel3_len = v,
            0xFF1C => self.channel3_vol = (v & 0x60) >> 5,
            0xFF1D => self.channel3_freq = self.channel3_freq & 0xFF00 | v as u32,
            0xFF1E => {
                self.channel3_freq = self.channel3_freq & 0x00FF | (((v & 0x7) as u32) << 8);
                self.channel3_started = v & 0x80 == 0x80;
                self.channel3_uselen = v & 0x40 == 0x40;
                self.channel3_wave_idx = 31;
                self.channel3_freq_div = 0;
                self.hz256 = 0;
            }
            0xFF30 ... 0xFF3F => self.waveram[a as usize - 0xFF30] = v,
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
        self.bliptime += cycles;
        self.hz256 += cycles;
        let trigger256 = if self.hz256 >= (1 << 22) / 256 {
            self.hz256 -= (1 << 22) / 256;
            true
        }
        else {
            false
        };
        if self.channel3_started && self.channel3_on {
            //let rfreq = ((2048 - (self.channel3_freq as u32)) << 5);
            let rfreq = 64 * (2048 - (self.channel3_freq as u32));
            self.channel3_freq_div += cycles;
            if self.channel3_uselen && trigger256 {
                self.channel3_len = self.channel3_len.wrapping_sub(1);
                if self.channel3_len == 0 {
                    self.channel3_started = false;
                }
            }
            if self.channel3_freq_div > rfreq {
                self.channel3_freq_div -= rfreq;
                self.channel3_wave_idx = (self.channel3_wave_idx + 1) % 32;
                let sample = {
                    let ramitem = self.waveram[self.channel3_wave_idx / 2];
                    let shifted = if self.channel3_wave_idx % 2 == 0 {
                        ramitem >> 4
                    }
                    else {
                        ramitem & 0x0F
                    };
                    shifted as i8
                };
                let volmul = match self.channel3_vol {
                    1 => 1.0,
                    2 => 0.5,
                    3 => 0.25,
                    _ => 0.0,
                };
                let newblip = ((sample as f64 / 7.5 - 1.0) * volmul * 5000.0) as i32 - self.blipval;
                let time = self.bliptime;
                if self.blip.is_some() {
                    self.blip().add_delta(time, newblip);
                    self.blipval += newblip;
                }
            }
        }
        else if self.blipval != 0 && self.blip.is_some() {
            let time = self.bliptime;
            let newblip = -self.blipval;
            self.blip().add_delta(time, newblip);
            self.blipval = 0;
        }
        if self.bliptime >= (1 << 18) && self.blip.is_some() {
            self.blip().end_frame(1 << 18);
            self.bliptime -= 1 << 18;
            self.play_blipbuf();
        }
    }

    fn play_blipbuf(&mut self) {
        let channels_len = self.channel().format().channels.len();

        while self.blip().samples_avail() > 0 {
            let buf = &mut [0; 1 << 13];
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
                self.channel().play();
            }
        }
    }
}

fn get_channel() -> Option<cpal::Voice> {
    if cpal::get_endpoints_list().count() == 0 { return None; }

    let endpoint = try_opt!(cpal::get_default_endpoint());
    let format = try_opt!(endpoint.get_supported_formats_list().ok().and_then(|mut v| v.next()));
    let channel = try_opt!(cpal::Voice::new(&endpoint, &format).ok());
    Some(channel)
}
