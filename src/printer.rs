struct GbPrinter {
    status: u8,
    state: u32,
    data: [u8; 0x280*9],
    packet: [u8; 0x400],
    count: usize,
    datacount: usize,
    datasize: usize,
    result: u8,
}

impl GbPrinter {
    fn new() -> GbPrinter {
        GbPrinter {
            status: 0,
            state: 0,
            data: [0; 0x280*9],
            packet: [0; 0x400],
            count: 0,
            datacount: 0,
            datasize: 0,
            result: 0,
        }
    }

    fn check_crc(&self) -> bool {
        let mut crc = 0;
        for i in 2..(6 + self.datasize) {
            crc += self.packet[i] as u16;
        }

        let msgcrc = self.packet[6 + self.datasize] as u16
            + ((self.packet[7 + self.datasize] as u16) << 8);

        crc == msgcrc
    }

    fn reset(&mut self) {
        self.state = 0;
        self.datasize = 0;
        self.datacount = 0;
        self.count = 0;
        self.status = 0;
        self.result = 0;
    }

    fn show(&self) {
        unimplemented!();
    }

    fn receive(&mut self) {
        if self.packet[3] != 0 {
            let mut dataidx = 6;
            let mut destidx = self.datacount;
            let mut len = 0;

            while len < self.datasize {
                let control = self.packet[dataidx];
                dataidx += 1;

                if control & 0x80 != 0 {
                    let curlen = ((control & 0x7F) + 2) as usize;
                    for i in 0..curlen {
                        self.data[destidx + i] = self.packet[dataidx];
                    }
                    dataidx += 1;
                    len += curlen;
                    destidx += curlen;
                }
                else {
                    let curlen = (control + 1) as usize;
                    for i in 0..curlen {
                        self.data[destidx + i] = self.packet[dataidx + i];
                    }
                    destidx += curlen;
                    dataidx += curlen;
                    len += curlen as usize;
                }
            }
        }
        else {
            for i in 0..self.datasize {
                self.data[self.datacount + i] = self.packet[6 + i];
            }
            self.datacount += self.datasize;
        }
    }

    fn command(&mut self) {
        match self.packet[2] {
            0x01 => {
                self.datacount = 0;
                self.status = 0;
            },
            0x02 => {
                self.show();
            },
            0x04 => {
                self.receive();
            },
            _ => (),
        }
    }

    fn send(&mut self, v: u8) -> u8 {
        self.packet[self.count] = v;
        self.count += 1;

        match self.state {
            0 => {
                self.count = 0;
                if v == 0x88 {
                    self.packet[self.count] = v;
                    self.count += 1;
                    self.state = 1;
                }
                else {
                    self.reset();
                }
            },
            1 => {
                if v == 0x33 {
                    self.packet[self.count] = v;
                    self.count += 1;
                    self.state = 2;
                }
                else {
                    self.reset();
                }
            },
            2 => {
                self.packet[self.count] = v;
                self.count += 1;
                if self.count == 6 {
                    self.state = 3;
                    self.datasize = self.packet[4] as usize + ((self.packet[5] as usize) << 8);
                }
            },
            3 => {
                if self.datasize != 0 {
                    self.packet[self.count] = v;
                    self.count += 1;
                    if self.count == self.datasize + 6 {
                        self.state = 4;
                    }
                }
                else {
                    self.state = 4;
                    return self.send(v);
                }
            },
            4 => {
                self.packet[self.count] = v;
                self.count += 1;
                self.state = 5;
            },
            5 => {
                self.packet[self.count] = v;
                self.count += 1;
                if self.check_crc() {
                    self.command();
                }
                self.state = 6;
            },
            6 => {
                self.packet[self.count] = v;
                self.count += 1;
                self.result = 0x81;
                self.state = 7;
            },
            7 => {
                self.packet[self.count] = v;
                self.count += 1;
                self.result = self.status;
                self.state = 0;
                self.count = 1;
            },
            _ => {
                self.reset()
            },
        }
        self.result
    }
}
