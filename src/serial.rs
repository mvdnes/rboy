use serde::{Deserialize, Serialize};

pub trait SerialCallback {
    fn call(&mut self, value: u8) -> Option<u8>;
}

#[derive(Serialize, Deserialize)]
pub struct Serial {
    data: u8,
    control: u8,
    #[serde(skip)]
    callback: Option<Box<dyn SerialCallback>>,
    pub interrupt: u8,
}

impl Serial {
    pub fn new_with_callback(cb: Box<dyn SerialCallback>) -> Serial {
        Serial {
            data: 0,
            control: 0,
            callback: Some(cb),
            interrupt: 0,
        }
    }

    pub fn wb(&mut self, a: u16, v: u8) {
        match a {
            0xFF01 => self.data = v,
            0xFF02 => {
                self.control = v;
                if v & 0x81 == 0x81 {
                    if let Some(callback) = &mut self.callback {
                        if let Some(result) = callback.call(self.data) {
                            self.data = result;
                            self.interrupt = 0x8;
                        }
                    }
                }
            }
            _ => panic!("Serial does not handle address {:4X} (write)", a),
        };
    }

    pub fn rb(&self, a: u16) -> u8 {
        match a {
            0xFF01 => self.data,
            0xFF02 => self.control | 0b01111110,
            _ => panic!("Serial does not handle address {:4X} (read)", a),
        }
    }

    pub fn set_callback(&mut self, cb: Box<dyn SerialCallback>) {
        self.callback = Some(cb);
    }

    pub fn unset_callback(&mut self) {
        self.callback = None;
    }
}

impl Serial {
    pub fn new() -> Serial {
        Serial {
            data: 0,
            control: 0,
            callback: None,
            interrupt: 0,
        }
    }
}
