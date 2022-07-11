use blip_buf::BlipBuf;

const WAVE_PATTERN : [[i32; 8]; 4] = [[-1,-1,-1,-1,1,-1,-1,-1],[-1,-1,-1,-1,1,1,-1,-1],[-1,-1,1,1,1,1,-1,-1],[1,1,1,1,-1,-1,1,1]];
const CLOCKS_PER_SECOND : u32 = 1 << 22;
const OUTPUT_SAMPLE_COUNT : usize = 2000; // this should be less than blip_buf::MAX_FRAME

pub trait AudioPlayer : Send {
    fn play(&mut self, left_channel: &[f32], right_channel: &[f32]);
    fn samples_rate(&self) -> u32;
    fn underflowed(&self) -> bool;
}

struct VolumeEnvelope {
    period : u8,
    goes_up : bool,
    delay : u8,
    initial_volume : u8,
    volume : u8,
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

    fn rb(&self, a: u16) -> u8 {
        match a {
            0xFF12 | 0xFF17 | 0xFF21 => {
                ((self.initial_volume & 0xF) << 4) |
                if self.goes_up { 0x08 } else { 0 } |
                (self.period & 0x7)
            },
            _ => unimplemented!(),
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
            },
            _ => (),
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
    active: bool,
    dac_enabled: bool,
    duty : u8,
    phase : u8,
    length: u8,
    length_enabled : bool,
    frequency: u16,
    period: u32,
    last_amp: i32,
    delay: u32,
    has_sweep : bool,
    sweep_frequency: u16,
    sweep_delay: u8,
    sweep_period: u8,
    sweep_shift: u8,
    sweep_frequency_increase: bool,
    volume_envelope: VolumeEnvelope,
    blip: BlipBuf,
}

impl SquareChannel {
    fn new(blip: BlipBuf, with_sweep: bool) -> SquareChannel {
        SquareChannel {
            active: false,
            dac_enabled: false,
            duty: 1,
            phase: 1,
            length: 0,
            length_enabled: false,
            frequency: 0,
            period: 2048,
            last_amp: 0,
            delay: 0,
            has_sweep: with_sweep,
            sweep_frequency: 0,
            sweep_delay: 0,
            sweep_period: 0,
            sweep_shift: 0,
            sweep_frequency_increase: false,
            volume_envelope: VolumeEnvelope::new(),
            blip: blip,
        }
    }

    fn on(&self) -> bool {
        self.active
    }

    fn rb(&self, a: u16) -> u8 {
        match a {
            0xFF10 => {
                0x80 |
                ((self.sweep_period & 0x7) << 4) |
                if self.sweep_frequency_increase { 0x8 } else { 0 } |
                (self.sweep_shift & 0x7)
            },
            0xFF11 | 0xFF16 => {
                ((self.duty & 3) << 6) |
                0x3F
            },
            0xFF12 | 0xFF17 => {
                self.volume_envelope.rb(a)
            },
            0xFF13 | 0xFF18 => {
                0xFF
            }
            0xFF14 | 0xFF19 => {
                0x80 |
                if self.length_enabled { 0x40 } else { 0 } |
                0x3F
            },
            _ => unimplemented!(),
        }
    }

    fn wb(&mut self, a: u16, v: u8) {
        match a {
            0xFF10 => {
                self.sweep_period = (v >> 4) & 0x7;
                self.sweep_shift = v & 0x7;
                self.sweep_frequency_increase = v & 0x8 == 0x8;
            },
            0xFF11 | 0xFF16 => {
                self.duty = v >> 6;
                self.length = 64 - (v & 0x3F);
            },
            0xFF12 | 0xFF17 => {
                self.dac_enabled = v & 0xF8 != 0;
                self.active = self.active && self.dac_enabled;
            }
            0xFF13 | 0xFF18 => {
                self.frequency = (self.frequency & 0x0700) | (v as u16);
                self.calculate_period();
            },
            0xFF14 | 0xFF19 => {
                self.frequency = (self.frequency & 0x00FF) | (((v & 0b0000_0111) as u16) << 8);
                self.calculate_period();
                self.length_enabled = v & 0x40 == 0x40;

                if self.dac_enabled && v & 0x80 == 0x80 {
                    self.active = true;
                    if self.length == 0 {
                        self.length = 64;
                    }
                    self.sweep_frequency = self.frequency;
                    if self.has_sweep && self.sweep_period > 0 && self.sweep_shift > 0 {
                        self.sweep_delay = 1;
                        self.step_sweep();
                    }
                }
            },
            _ => (),
        }
        self.volume_envelope.wb(a, v);
    }

    fn calculate_period(&mut self) {
        if self.frequency > 2048 { self.period = 0; }
        else { self.period = (2048 - self.frequency as u32) * 4; }
    }

    // This assumes no volume or sweep adjustments need to be done in the meantime
    fn run(&mut self, start_time: u32, end_time: u32) {
        if !self.active|| self.period == 0 || self.volume_envelope.volume == 0 {
            if self.last_amp != 0 {
                self.blip.add_delta(start_time, -self.last_amp);
                self.last_amp = 0;
                self.delay = 0;
            }
        }
        else {
            let mut time = start_time + self.delay;
            let pattern = WAVE_PATTERN[self.duty as usize];
            let vol = self.volume_envelope.volume as i32;

            while time < end_time {
                let amp = vol * pattern[self.phase as usize];
                if amp != self.last_amp {
                    self.blip.add_delta(time, amp - self.last_amp);
                    self.last_amp = amp;
                }
                time += self.period;
                self.phase = (self.phase + 1) % 8;
            }

            // next time, we have to wait an additional delay timesteps
            self.delay = time - end_time;
        }
    }

    fn step_length(&mut self) {
        if self.length_enabled && self.length != 0 {
            self.length -= 1;
            if self.length == 0 {
                self.active = false;
            }
        }
    }

    fn step_sweep(&mut self) {
        if !self.has_sweep || self.sweep_period == 0 { return; }

        if self.sweep_delay > 1 {
            self.sweep_delay -= 1;
        }
        else {
            self.sweep_delay = self.sweep_period;
            self.frequency = self.sweep_frequency;
            if self.frequency == 2048 {
                self.active = false;
            }
            self.calculate_period();

            let offset = self.sweep_frequency >> self.sweep_shift;

            if self.sweep_frequency_increase {
                // F ~ (2048 - f)
                // Increase in frequency means subtracting the offset
                if self.sweep_frequency <= offset {
                    self.sweep_frequency = 0;
                }
                else {
                    self.sweep_frequency -= offset;
                }
            }
            else {
                if self.sweep_frequency >= 2048 - offset {
                    self.sweep_frequency = 2048;
                }
                else {
                    self.sweep_frequency += offset;
                }
            }
        }
    }
}

struct WaveChannel {
    active: bool,
    dac_enabled : bool,
    length: u16,
    length_enabled : bool,
    frequency: u16,
    period: u32,
    last_amp: i32,
    delay: u32,
    volume_shift: u8,
    waveram: [u8; 32],
    current_wave: u8,
    blip: BlipBuf,
}

impl WaveChannel {
    fn new(blip: BlipBuf) -> WaveChannel {
        WaveChannel {
            active: false,
            dac_enabled: false,
            length: 0,
            length_enabled: false,
            frequency: 0,
            period: 2048,
            last_amp: 0,
            delay: 0,
            volume_shift: 0,
            waveram: [0; 32],
            current_wave: 0,
            blip: blip,
        }
    }

    fn rb(&self, a: u16) -> u8 {
        match a {
            0xFF1A => {
                (if self.dac_enabled { 0x80 } else { 0 }) |
                0x7F
            },
            0xFF1B => 0xFF,
            0xFF1C => {
                0x80 |
                ((self.volume_shift & 0b11) << 5) |
                0x1F
            },
            0xFF1D => 0xFF,
            0xFF1E => {
                0x80 |
                if self.length_enabled { 0x40 } else { 0 } |
                0x3F
            },
            0xFF30 ..= 0xFF3F => {
                (self.waveram[(a as usize - 0xFF30) * 2] << 4) |
                self.waveram[(a as usize - 0xFF30) * 2 + 1]
            },
            _ => unimplemented!(),
        }
    }

    fn wb(&mut self, a: u16, v: u8) {
        match a {
            0xFF1A => {
                self.dac_enabled = (v & 0x80) == 0x80;
                self.active = self.active && self.dac_enabled;
            }
            0xFF1B => self.length = 256 - (v as u16),
            0xFF1C => self.volume_shift = (v >> 5) & 0b11,
            0xFF1D => {
                self.frequency = (self.frequency & 0x0700) | (v as u16);
                self.calculate_period();
            }
            0xFF1E => {
                self.frequency = (self.frequency & 0x00FF) | (((v & 0b111) as u16) << 8);
                self.calculate_period();
                self.length_enabled = v & 0x40 == 0x40;
                if v & 0x80 == 0x80 && self.dac_enabled {
                    self.active = true;
                    if self.length == 0 {
                        self.length = 256;
                    }
                    self.current_wave = 0;
                    self.delay = 0;
                }
            },
            0xFF30 ..= 0xFF3F => {
                self.waveram[(a as usize - 0xFF30) * 2] = v >> 4;
                self.waveram[(a as usize - 0xFF30) * 2 + 1] = v & 0xF;
            },
            _ => (),
        }
    }

    fn calculate_period(&mut self) {
        if self.frequency > 2048 { self.period = 0; }
        else { self.period = (2048 - self.frequency as u32) * 2; }
    }

    fn on(&self) -> bool {
        self.active
    }

    fn run(&mut self, start_time: u32, end_time: u32) {
        if !self.active || self.period == 0 {
            if self.last_amp != 0 {
                self.blip.add_delta(start_time, -self.last_amp);
                self.last_amp = 0;
                self.delay = 0;
            }
        }
        else {
            let mut time = start_time + self.delay;

            // A sample may be muted, 100%, 50% or 25%.
            // To avoid loss of precision, the WaveChannel will output
            // i32 samples at 4x the usual amplitude. This will be taken
            // into account when mixing all the samples.
            let volshift = match self.volume_shift {
                0 => 4 + 2,  // to mute a 4 bit sample mutiplied by 2^2
                1 => 0,
                2 => 1,
                3 => 2,
                _ => unreachable!(),
            };

            while time < end_time {
                let sample = self.waveram[self.current_wave as usize];

                // shifted by 2 so that 25% does not lose precision
                let amp = ((sample << 2) >> volshift) as i32;

                if amp != self.last_amp {
                    self.blip.add_delta(time, amp - self.last_amp);
                    self.last_amp = amp;
                }

                time += self.period;
                self.current_wave = (self.current_wave + 1) % 32;
            }

            // next time, we have to wait an additional delay timesteps
            self.delay = time - end_time;
        }
    }

    fn step_length(&mut self) {
        if self.length_enabled && self.length != 0 {
            self.length -= 1;
            if self.length == 0 {
                self.active = false;
            }
        }
    }
}

struct NoiseChannel {
    active: bool,
    dac_enabled: bool,
    reg_ff22: u8,
    length: u8,
    length_enabled: bool,
    volume_envelope: VolumeEnvelope,
    period: u32,
    shift_width: u8,
    state: u16,
    delay: u32,
    last_amp: i32,
    blip: BlipBuf,
}

impl NoiseChannel {
    fn new(blip: BlipBuf) -> NoiseChannel {
        NoiseChannel {
            active: false,
            dac_enabled: false,
            reg_ff22: 0,
            length: 0,
            length_enabled: false,
            volume_envelope: VolumeEnvelope::new(),
            period: 2048,
            shift_width: 14,
            state: 1,
            delay: 0,
            last_amp: 0,
            blip: blip,
        }
    }

    fn rb(&self, a: u16) -> u8 {
        match a {
            0xFF20 => { 0xFF },
            0xFF21 => { self.volume_envelope.rb(a) },
            0xFF22 => {
                self.reg_ff22
            },
            0xFF23 => {
                0x80 |
                if self.length_enabled { 0x40 } else { 0 } |
                0x3F
            },
            _ => unimplemented!(),
        }
    }

    fn wb(&mut self, a: u16, v: u8) {
        match a {
            0xFF20 => self.length = 64 - (v & 0x3F),
            0xFF21 => {
                self.dac_enabled = v & 0xF8 != 0;
                self.active = self.active && self.dac_enabled;
            },
            0xFF22 => {
                self.reg_ff22 = v;
                self.shift_width = if v & 8 == 8 { 6 } else { 14 };
                let freq_div = match v & 7 {
                    0 => 8,
                    n => (n as u32 + 1) * 16,
                };
                self.period = freq_div << (v >> 4);
            },
            0xFF23 => {
                self.length_enabled = v & 0x40 == 0x40;
                if self.dac_enabled && v & 0x80 == 0x80 {
                    self.active = true;
                    if self.length == 0 {
                        self.length = 64;
                    }
                    self.state = 0xFF;
                    self.delay = 0;
                }
            },
            _ => (),
        }
        self.volume_envelope.wb(a, v);
    }

    fn on(&self) -> bool {
        self.active
    }

    fn run(&mut self, start_time: u32, end_time: u32) {
        if !self.active || self.volume_envelope.volume == 0 {
            if self.last_amp != 0 {
                self.blip.add_delta(start_time, -self.last_amp);
                self.last_amp = 0;
                self.delay = 0;
            }
        }
        else {
            let mut time = start_time + self.delay;
            while time < end_time {
                let oldstate = self.state;
                self.state <<= 1;
                let bit = ((oldstate >> self.shift_width) ^ (self.state >> self.shift_width)) & 1;
                self.state |= bit;

                let amp = match (oldstate >> self.shift_width) & 1 {
                    0 => -(self.volume_envelope.volume as i32),
                    _ => self.volume_envelope.volume as i32,
                };

                if self.last_amp != amp {
                    self.blip.add_delta(time, amp - self.last_amp);
                    self.last_amp = amp;
                }

                time += self.period;
            }
            self.delay = time - end_time;
        }
    }

    fn step_length(&mut self) {
        if self.length_enabled && self.length != 0 {
            self.length -= 1;
            if self.length == 0 {
                self.active = false;
            }
        }
    }
}

pub struct Sound {
    on: bool,
    time: u32,
    prev_time: u32,
    next_time: u32,
    time_divider: u8,
    output_period: u32,
    channel1: SquareChannel,
    channel2: SquareChannel,
    channel3: WaveChannel,
    channel4: NoiseChannel,
    volume_left: u8,
    volume_right: u8,
    reg_vin_to_so: u8,
    reg_ff25: u8,
    need_sync: bool,
    player: Box<dyn AudioPlayer>,
}

impl Sound {
    pub fn new(player: Box<dyn AudioPlayer>) -> Sound {
        let blipbuf1 = create_blipbuf(player.samples_rate());
        let blipbuf2 = create_blipbuf(player.samples_rate());
        let blipbuf3 = create_blipbuf(player.samples_rate());
        let blipbuf4 = create_blipbuf(player.samples_rate());

        let output_period = (OUTPUT_SAMPLE_COUNT as u64 * CLOCKS_PER_SECOND as u64) / player.samples_rate() as u64;

        Sound {
            on: false,
            time: 0,
            prev_time: 0,
            next_time: CLOCKS_PER_SECOND / 256,
            time_divider: 0,
            output_period: output_period as u32,
            channel1: SquareChannel::new(blipbuf1, true),
            channel2: SquareChannel::new(blipbuf2, false),
            channel3: WaveChannel::new(blipbuf3),
            channel4: NoiseChannel::new(blipbuf4),
            volume_left: 7,
            volume_right: 7,
            reg_vin_to_so: 0x00,
            reg_ff25: 0x00,
            need_sync: false,
            player: player,
        }
    }

   pub fn rb(&mut self, a: u16) -> u8 {
        self.run();
        let v = match a {
            0xFF10 ..= 0xFF14 => self.channel1.rb(a),
            0xFF15 => 0xFF,
            0xFF16 ..= 0xFF19 => self.channel2.rb(a),
            0xFF1A ..= 0xFF1E => self.channel3.rb(a),
            0xFF1F => 0xFF,
            0xFF20 ..= 0xFF23 => self.channel4.rb(a),
            0xFF24 => ((self.volume_right & 7) << 4) | (self.volume_left & 7) | self.reg_vin_to_so,
            0xFF25 => self.reg_ff25,
            0xFF26 => (
                if self.on { 0x80 } else { 0x00 } |
                0x70 |
                if self.channel4.on() { 0x8 } else { 0x0 } |
                if self.channel3.on() { 0x4 } else { 0x0 } |
                if self.channel2.on() { 0x2 } else { 0x0 } |
                if self.channel1.on() { 0x1 } else { 0x0 }),
            0xFF27 ..= 0xFF2F => 0xFF,
            0xFF30 ..= 0xFF3F => self.channel3.rb(a),
            _ => 0,
        };
        return v;
    }

    pub fn wb(&mut self, a: u16, v: u8) {
        if a != 0xFF26 && !self.on { return; }
        self.run();
        match a {
            0xFF10 ..= 0xFF14 => self.channel1.wb(a, v),
            0xFF16 ..= 0xFF19 => self.channel2.wb(a, v),
            0xFF1A ..= 0xFF1E => self.channel3.wb(a, v),
            0xFF20 ..= 0xFF23 => self.channel4.wb(a, v),
            0xFF24 => {
                self.volume_left = v & 0x7;
                self.volume_right = (v >> 4) & 0x7;
                self.reg_vin_to_so = v & 0x88;
            }
            0xFF25 => self.reg_ff25 = v,
            0xFF26 => {
                let turn_on = v & 0x80 == 0x80;
                if self.on && turn_on == false {
                    for i in 0xFF10..=0xFF25 {
                        self.wb(i, 0);
                    }
                }
                self.on = turn_on;
            }
            0xFF30 ..= 0xFF3F => self.channel3.wb(a, v),
            _ => (),
        }
    }

    pub fn do_cycle(&mut self, cycles: u32)
    {
        if !self.on { return; }

        self.time += cycles;

        if self.time >= self.output_period {
            self.do_output();
        }
    }

    pub fn sync(&mut self) {
        self.need_sync = true;
    }

    fn do_output(&mut self) {
        self.run();
        debug_assert!(self.time == self.prev_time);
        self.channel1.blip.end_frame(self.time);
        self.channel2.blip.end_frame(self.time);
        self.channel3.blip.end_frame(self.time);
        self.channel4.blip.end_frame(self.time);
        self.next_time -= self.time;
        self.time = 0;
        self.prev_time = 0;

        if !self.need_sync || self.player.underflowed() {
            self.need_sync = false;
            self.mix_buffers();
        }
        else {
            // Prevent the BlipBuf's from filling up and triggering an assertion
            self.clear_buffers();
        }
    }

    fn run(&mut self) {
        while self.next_time <= self.time {
            self.channel1.run(self.prev_time, self.next_time);
            self.channel2.run(self.prev_time, self.next_time);
            self.channel3.run(self.prev_time, self.next_time);
            self.channel4.run(self.prev_time, self.next_time);

            self.channel1.step_length();
            self.channel2.step_length();
            self.channel3.step_length();
            self.channel4.step_length();

            if self.time_divider == 0 {
                self.channel1.volume_envelope.step();
                self.channel2.volume_envelope.step();
                self.channel4.volume_envelope.step();
            }
            else if self.time_divider & 1 == 1 {
                self.channel1.step_sweep();
            }

            self.time_divider = (self.time_divider + 1) % 4;
            self.prev_time = self.next_time;
            self.next_time += CLOCKS_PER_SECOND / 256;
        }

        if self.prev_time != self.time {
            self.channel1.run(self.prev_time, self.time);
            self.channel2.run(self.prev_time, self.time);
            self.channel3.run(self.prev_time, self.time);
            self.channel4.run(self.prev_time, self.time);

            self.prev_time = self.time;
        }
    }

    fn mix_buffers(&mut self) {
        let sample_count = self.channel1.blip.samples_avail() as usize;
        debug_assert!(sample_count == self.channel2.blip.samples_avail() as usize);
        debug_assert!(sample_count == self.channel3.blip.samples_avail() as usize);
        debug_assert!(sample_count == self.channel4.blip.samples_avail() as usize);

        let mut outputted = 0;

        let left_vol = (self.volume_left as f32 / 7.0) * (1.0 / 15.0) * 0.25;
        let right_vol = (self.volume_right as f32 / 7.0) * (1.0 / 15.0) * 0.25;

        while outputted < sample_count {
            let buf_left = &mut [0f32; OUTPUT_SAMPLE_COUNT + 10];
            let buf_right = &mut [0f32; OUTPUT_SAMPLE_COUNT + 10];
            let buf = &mut [0i16; OUTPUT_SAMPLE_COUNT + 10];

            let count1 = self.channel1.blip.read_samples(buf, false);
            for (i, v) in buf[..count1].iter().enumerate() {
                if self.reg_ff25 & 0x01 == 0x01 {
                    buf_left[i] += *v as f32 * left_vol;
                }
                if self.reg_ff25 & 0x10 == 0x10 {
                    buf_right[i] += *v as f32 * right_vol;
                }
            }

            let count2 = self.channel2.blip.read_samples(buf, false);
            for (i, v) in buf[..count2].iter().enumerate() {
                if self.reg_ff25 & 0x02 == 0x02 {
                    buf_left[i] += *v as f32 * left_vol;
                }
                if self.reg_ff25 & 0x20 == 0x20 {
                    buf_right[i] += *v as f32 * right_vol;
                }
            }

            // channel3 is the WaveChannel, that outputs samples with a 4x
            // increase in amplitude in order to avoid a loss of precision.
            let count3 = self.channel3.blip.read_samples(buf, false);
            for (i, v) in buf[..count3].iter().enumerate() {
                if self.reg_ff25 & 0x04 == 0x04 {
                    buf_left[i] += ((*v as f32) / 4.0) * left_vol;
                }
                if self.reg_ff25 & 0x40 == 0x40 {
                    buf_right[i] += ((*v as f32) / 4.0) * right_vol;
                }
            }

            let count4 = self.channel4.blip.read_samples(buf, false);
            for (i, v) in buf[..count4].iter().enumerate() {
                if self.reg_ff25 & 0x08 == 0x08 {
                    buf_left[i] += *v as f32 * left_vol;
                }
                if self.reg_ff25 & 0x80 == 0x80 {
                    buf_right[i] += *v as f32 * right_vol;
                }
            }

            debug_assert!(count1 == count2);
            debug_assert!(count1 == count3);
            debug_assert!(count1 == count4);

            self.player.play(&buf_left[..count1], &buf_right[..count1]);

            outputted += count1;
        }
    }

    fn clear_buffers(&mut self) {
        self.channel1.blip.clear();
        self.channel2.blip.clear();
        self.channel3.blip.clear();
        self.channel4.blip.clear();
    }
}

fn create_blipbuf(samples_rate: u32) -> BlipBuf {
    let mut blipbuf = BlipBuf::new(samples_rate);
    blipbuf.set_rates(CLOCKS_PER_SECOND as f64, samples_rate as f64);
    blipbuf
}
