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
    channel3_freq: u16,
    channel3_counter: Counter,
    voice: Option<cpal::Voice>,
    blip: Option<BlipBuf>,
}

enum Counter {
    Stopped,
    Continuous,
    Timed(u8),
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
                bb.set_rates(0x400000 as f64, v.format().samples_rate.0 as f64);
                bb
            });

        Sound {
            waveram: [0; 16],
            channel3_on: false,
            channel3_len: 0,
            channel3_vol: 0,
            channel3_freq: 0,
            channel3_counter: Counter::Stopped,
            voice: voice,
            blip: blipbuf,
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
            0xFF1A => self.channel3_on = v & 0x80 == 0x80,
            0xFF1B => self.channel3_len = v,
            0xFF1C => self.channel3_vol = (v >> 5) & 0x3,
            0xFF1D => self.channel3_freq = self.channel3_freq & 0xFF00 | v as u16,
            0xFF1E => {
                self.channel3_freq = self.channel3_freq & 0x00FF | (((v & 0x7) as u16) << 8);
                match v & 0b_1100_0000 {
                    0b_1000_0000 => self.channel3_counter = Counter::Continuous,
                    0b_1100_0000 => self.channel3_counter = Counter::Timed(self.channel3_len),
                    _ => (),
                }
            }
            0xFF30 ... 0xFF3F => self.waveram[a as usize - 0xFF30] = v,
            _ => (),
        }
    }

    pub fn do_cycle(&mut self, _cycles: u32)
    {
        // To be implemented
    }
}

fn get_channel() -> Option<cpal::Voice> {
    if cpal::get_endpoints_list().count() == 0 { return None; }
    let endpoint = try_opt!(cpal::get_default_endpoint());
    let format = try_opt!(endpoint.get_supported_formats_list().ok().and_then(|mut v| v.next()));
    let channel = try_opt!(cpal::Voice::new(&endpoint, &format).ok());
    Some(channel)
}

fn play_blipbuf(channel: &mut cpal::Voice, blip: &mut BlipBuf) {
    let channels_len = channel.format().channels.len();

    while blip.samples_avail() > 0 {
        let buf = &mut [0; 1024];
        let count = blip.read_samples(buf, false);
        let blipbuf = &buf[0..count];
        let mut done = 0;
        let mut lastdone = count;

        while lastdone != done && done < count {
            lastdone = done;
            let channelbuf = &blipbuf[done..];
            match channel.append_data(channelbuf.len()) {
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
            channel.play();
        }
    }
}
