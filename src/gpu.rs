
static VRAM_SIZE: uint = 0x2000;
static VOAM_SIZE: uint = 0xA0;
static SCREEN_W: uint = 160;
static SCREEN_H: uint = 144;

pub struct GPU {
	priv mode: u8,
	priv modeclock: uint,
	priv line: u8,
	priv lyc: u8,
	priv lcd_on: bool,
	priv win_tilemapbase: u16,
	priv win_on: bool,
	priv tilebase: u16,
	priv bg_tilemap: u16,
	priv sprite_size: uint,
	priv sprite_on: bool,
	priv bg_on: bool,
	priv lyc_inte: bool,
	priv m0_inte: bool,
	priv m1_inte: bool,
	priv m2_inte: bool,
	priv scy: u8,
	priv scx: u8,
	priv winy: u8,
	priv winx: u8,
	priv palbr: u8,
	priv pal0r: u8,
	priv pal1r: u8,
	priv palb: [u8,.. 4],
	priv pal0: [u8,.. 4],
	priv pal1: [u8,.. 4],
	priv vram: ~[u8,.. VRAM_SIZE],
	priv voam: ~[u8,.. VOAM_SIZE],
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
			lyc: 0,
			lcd_on: false,
			win_tilemapbase: 0x9C00,
			win_on: false,
			tilebase: 0x8000,
			bg_tilemap: 0x9C00,
			sprite_size: 8,
			sprite_on: false,
			bg_on: false,
			lyc_inte: false,
			m2_inte: false,
			m1_inte: false,
			m0_inte: false,
			scy: 0,
			scx: 0,
			winy: 0,
			winx: 0,
			palbr: 0,
			pal0r: 0,
			pal1r: 1,
			palb: [0,.. 4],
			pal0: [0,.. 4],
			pal1: [0,.. 4],
			vram: ~([0,.. VRAM_SIZE]),
			voam: ~([0,.. VOAM_SIZE]),
			data: ~([0,.. SCREEN_W * SCREEN_H * 3]),
			updated: false,
			interrupt: 0,
		}
	}

	pub fn cycle(&mut self, ticks: uint) {
		if !self.lcd_on { return }
		
		self.modeclock += ticks;

		// Full line takes 114 ticks
		if self.modeclock >= 114 {
			self.modeclock -= 114;
			self.line = (self.line + 1) % 154;
			self.check_interrupt_lyc();

			// This is a VBlank line
			if self.line >= 144 && self.mode != 1 {
				self.change_mode(1);
			}
		}

		// This is a normal line
		if self.line < 144 {
			if self.modeclock <= 20 {
				if self.mode != 2 { self.change_mode(2); }
			} else if self.modeclock <= (20 + 43) {
				if self.mode != 3 { self.change_mode(3); }
			} else { // the remaining 51
				if self.mode != 0 { self.change_mode(0); }
			}
		}
	}

	fn check_interrupt_lyc(&mut self) {
		if self.lyc_inte && self.line == self.lyc {
			self.interrupt |= 0x02;
		}
	}

	fn change_mode(&mut self, mode: u8) {
		self.mode = mode;

		if match self.mode {
			0 => {
				self.renderscan();
				self.m0_inte
			},
			1 => {
				self.interrupt |= 0x01;
				self.updated = true;
				self.m1_inte
			},
			2 => self.m2_inte,
			_ => false,
		} {
			self.interrupt |= 0x02;
		}
	}

	pub fn rb(&self, a: u16) -> u8 {
		match a {
			0x8000 .. 0x9FFF => self.vram[a & 0x1FFF],
			0xFE00 .. 0xFE9F => self.voam[a - 0xFE00],
			0xFF40 => {
				(if self.lcd_on { 0x80 } else { 0 }) |
				(if self.win_tilemapbase == 0x9C00 { 0x40 } else { 0 }) |
				(if self.win_on { 0x20 } else { 0 }) |
				(if self.tilebase == 0x8000 { 0x10 } else { 0 }) |
				(if self.bg_tilemap == 0x9C00 { 0x08 } else { 0 }) |
				(if self.sprite_size == 16 { 0x04 } else { 0 }) |
				(if self.sprite_on { 0x02 } else { 0 }) |
				(if self.bg_on { 0x01 } else { 0 })
			},
			0xFF41 => {
				(if self.lyc_inte { 0x40 } else { 0 }) |
				(if self.m2_inte { 0x20 } else { 0 }) |
				(if self.m1_inte { 0x10 } else { 0 }) |
				(if self.m0_inte { 0x08 } else { 0 }) |
				(if self.line == self.lyc { 0x04 } else { 0 }) |
				self.mode
			},
			0xFF42 => self.scy,
			0xFF43 => self.scx,
			0xFF44 => self.line,
			0xFF45 => self.lyc,
			0xFF46 => 0, // Write only
			0xFF47 => self.palbr,
			0xFF48 => self.pal0r,
			0xFF49 => self.pal1r,
			0xFF4A => self.winy,
			0xFF4B => self.winx,
			_ => fail!("GPU does not handle read {:04X}", a),
		}
	}

	pub fn wb(&mut self, a: u16, v: u8) {
		match a {
			0x8000 .. 0x9FFF => self.vram[a & 0x1FFF] = v,
			0xFE00 .. 0xFE9F => self.voam[a - 0xFE00] = v,
			0xFF40 => {
				let orig_lcd_on = self.lcd_on;
				self.lcd_on = v & 0x80 == 0x80;
				self.win_tilemapbase = if v & 0x40 == 0x40 { 0x9C00 } else { 0x9800 };
				self.win_on = v & 0x20 == 0x20;
				self.tilebase = if v & 0x10 == 0x10 { 0x8000 } else { 0x9000 };
				self.bg_tilemap = if v & 0x08 == 0x08 { 0x9C00 } else { 0x9800 };
				self.sprite_size = if v & 0x04 == 0x04 { 16 } else { 8 };
				self.sprite_on = v & 0x02 == 0x02;
				self.bg_on = v & 0x01 == 0x01;
				if !orig_lcd_on && self.lcd_on { self.modeclock = 0; self.line = 0; self.mode = 0; }
			},
			0xFF41 => {
				self.lyc_inte = v & 0x40 == 0x40;
				self.m2_inte = v & 0x20 == 0x20;
				self.m1_inte = v & 0x10 == 0x10;
				self.m0_inte = v & 0x08 == 0x08;
			},
			0xFF42 => self.scy = v,
			0xFF43 => self.scx = v,
			0xFF44 => {}, // Read-only
			0xFF45 => self.lyc = v,
			0xFF46 => fail!("0xFF46 should be handled by MMU"),
			0xFF47 => { self.palbr = v; self.update_pal(); },
			0xFF48 => { self.pal0r = v; self.update_pal(); },
			0xFF49 => { self.pal1r = v; self.update_pal(); },
			0xFF4A => self.winy = v,
			0xFF4B => self.winx = v,
			_ => fail!("GPU does not handle write {:04X}", a),
		}
	}

	fn update_pal(&mut self) {
		for i in range(0, 4) {
			self.palb[i] = match (self.palbr >> 2*i) & 3 {
				0 => 255,
				1 => 192,
				2 => 96,
				_ => 0,
			};
			self.pal0[i] = match (self.pal0r >> 2*i) & 3 {
				0 => 255,
				1 => 192,
				2 => 96,
				_ => 0,
			};
			self.pal1[i] = match (self.pal1r >> 2*i) & 3 {
				0 => 255,
				1 => 192,
				2 => 96,
				_ => 0,
			};
		}
	}

	fn renderscan(&mut self) {
		self.draw_bg();
		self.draw_win();
		self.draw_sprites();
	}

	fn setcolor(&mut self, x: uint, color: u8) {
		self.data[self.line as uint * SCREEN_W * 3 + x * 3 + 0] = color;
		self.data[self.line as uint * SCREEN_W * 3 + x * 3 + 1] = color;
		self.data[self.line as uint * SCREEN_W * 3 + x * 3 + 2] = color;
	}

	fn draw_bg(&mut self) {
		if !self.bg_on { return }

		let bgy = self.scy + self.line;
		let tiley = (bgy as u16 >> 3) & 31;
		for x in range(0, SCREEN_W) {
			let bgx = self.scx as uint + x;
			let tilex = (bgx as u16 >> 3) & 31;

			let tilenr: u8 = self.rb(self.bg_tilemap + tiley * 32 + tilex);
			let tileaddress = (self.tilebase as int
			+ (if self.tilebase == 0x8000 {
				tilenr as u16 as int
			} else {
				tilenr as i8 as int
			}) * 16) as u16;

			let b1 = self.rb(tileaddress + ((bgy as u16 & 0x07) * 2));
			let b2 = self.rb(tileaddress + ((bgy as u16 & 0x07) * 2) + 1);

			let xbit = bgx & 0x07;
			let colnr = if b1 & (1 << (7 - xbit)) != 0 { 1 } else { 0 }
				| if b2 & (1 << (7 - xbit)) != 0 { 2 } else { 0 };
			self.setcolor(x, self.palb[colnr]);
		}
	}

	fn draw_win(&mut self) {
		if !self.win_on { return }
		let winy = self.line as int - self.winy as int;
		if winy < 0 { return }

		let tiley: u16 = ((winy as u16) >> 3) & 31;

		for x in range(0, SCREEN_W) {
			let winx: int = - ((self.winx as int) - 7) + (x as int);
			if winx < 0 { continue }
			let tilex: u16 = ((winx as u16) >> 3) & 31;

			let tilenr: u8 = self.rb(self.win_tilemapbase + tiley * 32 + tilex);
			let tileaddress = (self.tilebase as int
			+ (if self.tilebase == 0x8000 {
				tilenr as u16 as int
			} else {
				tilenr as i8 as int
			}) * 16) as u16;

			let b1 = self.rb(tileaddress + ((winy as u16 & 0x07) * 2));
			let b2 = self.rb(tileaddress + ((winy as u16 & 0x07) * 2) + 1);

			let xbit = winx & 0x07;
			let colnr = if b1 & (1 << (7 - xbit)) != 0 { 1 } else { 0 }
				| if b2 & (1 << (7 - xbit)) != 0 { 2 } else { 0 };
			self.setcolor(x, self.palb[colnr]);
		}
	}

	fn draw_sprites(&mut self) {
		if !self.sprite_on { return }

		// TODO: limit of 10 sprites per line

		for i in range(0u16, 40) {
			let spriteaddr: u16 = 0xFE00 + i * 4;
			let spritey: int = self.rb(spriteaddr + 0) as u16 as int - 16;
			let spritex: int = self.rb(spriteaddr + 1) as u16 as int - 8;
			let tilenum: u16 = (self.rb(spriteaddr + 2) & (if self.sprite_size == 16 { 0xFE } else { 0xFF })) as u16;
			let flags: u8 = self.rb(spriteaddr + 3);
			let usepal1: bool = flags & (1 << 4) != 0;
			let xflip: bool = flags & (1 << 5) != 0;
			let yflip: bool = flags & (1 << 6) != 0;
			let belowbg: bool = flags & (1 << 7) != 0;

			let line = self.line as int;
			let sprite_size = self.sprite_size as int;

			if line < spritey || line >= spritey + sprite_size { continue }
			if spritex < -7 || spritex >= (SCREEN_W as int) { continue }

			let tiley: u16 = if yflip {
				(sprite_size - 1 - (line - spritey)) as u16
			} else {
				(line - spritey) as u16
			};
			
			let tileaddress = 0x8000u16 + tilenum * 16 + tiley * 2;
			let b1 = self.rb(tileaddress);
			let b2 = self.rb(tileaddress + 1);

			for x in range(0, 8) {
				if spritex + x < 0 || spritex + x >= (SCREEN_W as int) { continue }

				let xbit: u8 = 1 << (if xflip { x } else { 7 - x });
				let colnr: u8 = (if b1 & xbit != 0 { 1 } else { 0 }) |
					(if b2 & xbit != 0 { 2 } else { 0 });
				if colnr == 0 { continue }
				//TODO: draw belowbg if bg == 0 and flag is set
				let color = if usepal1 { self.pal1[colnr] } else { self.pal0[colnr] };

				self.setcolor((spritex + x) as uint, color);
			}
		}
	}
}
