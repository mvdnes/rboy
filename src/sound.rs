
pub struct Sound {
    data: [u8; 0x30],
}

impl Sound {
    pub fn new() -> Sound {
        Sound { data: [0; 0x30] }
    }

    pub fn rb(&self, a: u16) -> u8 {
        self.data[a as usize - 0xFF10]
    }

    pub fn wb(&mut self, a: u16, v: u8) {
        self.data[a as usize - 0xFF10] = v;
    }

    pub fn do_cycle(&mut self, _cycles: u32)
    {
        // To be implemented
    }
}
