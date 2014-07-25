use std::sync::Arc;
use std::collections::DList;
use spinlock::Spinlock;

pub struct Sound {
	data: [u8,.. 0x30],
	buffer: Option<Arc<Spinlock<DList<u8>>>>,
}

impl Sound {
	pub fn new() -> Sound {
		Sound { data: [0,.. 0x30], buffer: None }
	}

	pub fn rb(&self, a: u16) -> u8 {
		self.data[a as uint - 0xFF10]
	}

	pub fn wb(&mut self, a: u16, v: u8) {
		self.data[a as uint - 0xFF10] = v;
	}

	pub fn attach_buffer(&mut self, buffer: Arc<Spinlock<DList<u8>>>)
	{
		self.buffer = Some(buffer);
	}

	pub fn cycle(&mut self, cycles: uint)
	{
		let mut data = match self.buffer
		{
			None => return,
			Some(ref buffer) => buffer.lock(),
		};

		if data.len() > 10000 { return; }

		for _ in range(0, cycles)
		{
			data.push(0);
		}
	}
}
