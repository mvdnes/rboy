use cpal;
use blip_buf::BlipBuf;

pub struct Sound {
    waveram: [u8; 16],
    channel3_on: bool,
    channel3_len: u8,
    channel3_vol: u8,
    channel3_freq: u16,
    channel3_counter: Counter,
}

enum Counter {
    Stopped,
    Continuous,
    Timed(u8),
}

impl Sound {
    pub fn new() -> Sound {

        Sound {
            waveram: [0; 16],
            channel3_on: false,
            channel3_len: 0,
            channel3_vol: 0,
            channel3_freq: 0,
            channel3_counter: Counter::Stopped,
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
                match v & 0xC0 {
                    0x80 => self.channel3_counter = Counter::Continuous,
                    0xC0 => self.channel3_counter = Counter::Timed(self.channel3_len),
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
