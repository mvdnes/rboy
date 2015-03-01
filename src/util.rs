use std::io;

pub fn handle_io<T>(result: io::Result<T>, message: &str) -> Option<T>
{
    match result
    {
        Ok(ok) => { Some(ok) },
        Err(ref error) =>
        {
            error!("{}: {}", message, error);
            None
        },
    }
}

pub trait WriteIntExt {
    fn write_be_i64(&mut self, i64) -> io::Result<()>;
}

pub trait ReadIntExt {
    fn read_be_i64(&mut self) -> io::Result<i64>;
}

impl<W: io::Write> WriteIntExt for W {
    fn write_be_i64(&mut self, val: i64) -> io::Result<()> {
        let val = val as u64;
        let buf = [
            (val >> (7*8)) as u8,
            (val >> (6*8)) as u8,
            (val >> (5*8)) as u8,
            (val >> (4*8)) as u8,
            (val >> (3*8)) as u8,
            (val >> (2*8)) as u8,
            (val >> (1*8)) as u8,
            (val >> (0*8)) as u8,
        ];
        self.write_all(&buf)
    }
}

impl<R: io::Read> ReadIntExt for R {
    fn read_be_i64(&mut self) -> io::Result<i64> {
        let buf = &mut [0u8; 8];
        let mut idx = 0;
        while idx != buf.len() {
            match self.read(&mut buf[idx..]) {
                Ok(0) => return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Could not fetch required bytes", None)),
                Ok(v) => idx += v,
                Err(e) => return Err(e),
            }
        }
        let val =
            ((buf[0] as u64) << (7*8)) |
            ((buf[1] as u64) << (6*8)) |
            ((buf[2] as u64) << (5*8)) |
            ((buf[3] as u64) << (4*8)) |
            ((buf[4] as u64) << (3*8)) |
            ((buf[5] as u64) << (2*8)) |
            ((buf[6] as u64) << (1*8)) |
            ((buf[7] as u64) << (0*8));
        Ok(val as i64)
    }
}
