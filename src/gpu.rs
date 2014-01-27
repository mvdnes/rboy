
static SCREEN_W: uint = 160;
static SCREEN_H: uint = 144;

pub struct GPU {
	priv mode: u8,
	priv modeclock: uint,
	priv line: u8,
	data: ~[u8,.. SCREEN_W * SCREEN_H * 3],
	updated: bool,
	interrupt: u8,
}

impl GPU {
	pub fn new() -> GPU {
		GPU {
			mode: 0,
			modeclock: 0,
			line: 0,
			data: ~([0xFF,.. SCREEN_W * SCREEN_H * 3]),
			updated: false,
			interrupt: 0,
		}
	}

	pub fn cycle(&mut self, ticks: uint) {
		
	}
}
