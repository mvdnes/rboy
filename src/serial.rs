pub type SerialCallback<'a> = Box<FnMut(u8) -> u8 + 'a>;

fn noop(_: u8) -> u8 { 0 }

pub struct Serial<'a> {
    data: u8,
    control: u8,
    callback: SerialCallback<'a>,
}

impl<'a> Serial<'a>
{
    pub fn new_with_callback(cb: SerialCallback<'a>) -> Serial<'a>
    {
        Serial { data: 0, control: 0, callback: cb, }
    }

    pub fn wb(&mut self, a: u16, v: u8) {
        match a {
            0xFF01 => self.data = v,
            0xFF02 => {
                self.control = v;
                if v == 0x81 {
                    self.data = (self.callback)(self.data);
                }
            },
            _ => panic!("Serial does not handle address {:4X} (write)", a),
        };
    }

    pub fn rb(&self, a: u16) -> u8 {
        match a {
            0xFF01 => self.data,
            0xFF02 => self.control,
            _ => panic!("Serial does not handle address {:4X} (read)", a),
        }
    }

    pub fn set_callback(&mut self, cb: SerialCallback<'static>) {
        self.callback = cb;
    }

    pub fn unset_callback(&mut self) {
        self.callback = Box::new(noop) as SerialCallback;
    }
}

impl Serial<'static> {
    pub fn new() -> Serial<'static> {
        Serial { data: 0, control: 0, callback: Box::new(noop) as SerialCallback }
    }
}
