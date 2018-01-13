pub struct GbPrinter {
    status: u8,
    state: u32,
    data: [u8; 0x280*9],
    packet: [u8; 0x400],
    count: usize,
    datacount: usize,
    datasize: usize,
    result: u8,
    printcount: u8
}

impl GbPrinter {
    pub fn new() -> GbPrinter {
        GbPrinter {
            status: 0,
            state: 0,
            data: [0; 0x280*9],
            packet: [0; 0x400],
            count: 0,
            datacount: 0,
            datasize: 0,
            result: 0,
            printcount: 0,
        }
    }

    fn check_crc(&self) -> bool {
        let mut crc = 0u16;
        for i in 2..(6 + self.datasize) {
            crc = crc.wrapping_add(self.packet[i] as u16);
        }

        let msgcrc = (self.packet[6 + self.datasize] as u16).wrapping_add((self.packet[7 + self.datasize] as u16) << 8);

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

    fn show(&mut self) {
        match self._show() {
            Ok(filename) => println!("Print saved successfully to {}", filename),
            Err(e) => println!("Error saving print... {:?}", e),
        }
    }

    fn _show(&mut self) -> ::std::io::Result<String> {
        use std::fs::OpenOptions;
        use std::io::Write;

        let filename = format!("rboy_print_{:03}.pgm", self.printcount);
        self.printcount += 1;

        let image_height = self.datacount / 40;
        if image_height == 0 {
            return Ok(filename);
        }

        let mut f = OpenOptions::new().create(true).write(true).truncate(true).open(&filename)?;

        write!(f, "P5 160 {} 3\n", image_height)?;

        let palbyte = self.packet[8];
        let palette = [3 - ((palbyte >> 0) & 3), 3 - ((palbyte >> 2) & 3), 3 - ((palbyte >> 4) & 3), 3 - ((palbyte >> 6) & 3)];

        for y in 0..image_height {
            for x in 0..160 {
                let tilenumber = ((y >> 3) * 20) + (x >> 3);
                let tileoffset = tilenumber * 16 + (y & 7) * 2;
                let bx = 7 - (x & 7);

                let colourindex = ((self.data[tileoffset] >> bx) & 1) | (((self.data[tileoffset + 1] >> bx) << 1) & 2);

                f.write_all(&[palette[colourindex as usize]])?;
            }
        }

        Ok(filename)
    }

    fn receive(&mut self) {
        if self.packet[3] != 0 {
            let mut dataidx = 6;
            let mut destidx = self.datacount;

            while dataidx - 6 < self.datasize {
                let control = self.packet[dataidx];
                dataidx += 1;

                if control & 0x80 != 0 {
                    let curlen = ((control & 0x7F) + 2) as usize;
                    for i in 0..curlen {
                        self.data[destidx + i] = self.packet[dataidx];
                    }
                    dataidx += 1;
                    destidx += curlen;
                }
                else {
                    let curlen = (control + 1) as usize;
                    for i in 0..curlen {
                        self.data[destidx + i] = self.packet[dataidx + i];
                    }
                    destidx += curlen;
                    dataidx += curlen;
                }
            }

            self.datacount = destidx;
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

    pub fn send(&mut self, v: u8) -> u8 {
        self.packet[self.count] = v;
        self.count += 1;

        match self.state {
            0 => {
                if v == 0x88 {
                    self.state = 1;
                }
                else {
                    self.reset();
                }
            },
            1 => {
                if v == 0x33 {
                    self.state = 2;
                }
                else {
                    self.reset();
                }
            },
            2 => {
                if self.count == 6 {
                    self.datasize = self.packet[4] as usize + ((self.packet[5] as usize) << 8);
                    if self.datasize > 0 {
                        self.state = 3;
                    }
                    else {
                        self.state = 4;
                    }
                }
            },
            3 => {
                if self.count == self.datasize + 6 {
                    self.state = 4;
                }
            },
            4 => {
                self.state = 5;
            },
            5 => {
                if self.check_crc() {
                    self.command();
                }
                self.state = 6;
            },
            6 => {
                self.result = 0x81;
                self.state = 7;
            },
            7 => {
                self.result = self.status;
                self.state = 0;
                self.count = 0;
            },
            _ => {
                self.reset()
            },
        }
        self.result
    }
}
