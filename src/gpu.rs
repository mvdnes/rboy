
static VRAM_SIZE: uint = 0x4000;
static VOAM_SIZE: uint = 0xA0;
static SCREEN_W: uint = 160;
static SCREEN_H: uint = 144;

#[deriving(Eq)]
enum PrioType {
	Color0,
	PrioFlag,
	Normal,
}

pub struct GPU {
	priv mode: u8,
	priv modeclock: uint,
	priv line: u8,
	priv lyc: u8,
	priv lcd_on: bool,
	priv win_tilemap: u16,
	priv win_on: bool,
	priv tilebase: u16,
	priv bg_tilemap: u16,
	priv sprite_size: uint,
	priv sprite_on: bool,
	priv lcdc0: bool,
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
	priv cbgpal_inc: bool,
	priv cbgpal_ind: u8,
	priv cbgpal: [[[u8,.. 3],.. 4],.. 8],
	priv csprit_inc: bool,
	priv csprit_ind: u8,
	priv csprit: [[[u8,.. 3],.. 4],.. 8],
	priv vrambank: u16,
	data: ~[u8,.. SCREEN_W * SCREEN_H * 3],
	priv bgprio: ~[PrioType,.. SCREEN_W],
	updated: bool,
	interrupt: u8,
	gbmode: ::gbmode::GbMode,
}

impl GPU {
	pub fn new() -> GPU {
		GPU {
			mode: 0,
			modeclock: 0,
			line: 0,
			lyc: 0,
			lcd_on: false,
			win_tilemap: 0x9C00,
			win_on: false,
			tilebase: 0x8000,
			bg_tilemap: 0x9C00,
			sprite_size: 8,
			sprite_on: false,
			lcdc0: false,
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
			bgprio: ~([Normal,.. SCREEN_W]),
			updated: false,
			interrupt: 0,
			gbmode: ::gbmode::Classic,
			cbgpal_inc: false,
			cbgpal_ind: 0,
			cbgpal: [[[0u8,.. 3],.. 4],.. 8],
			csprit_inc: false,
			csprit_ind: 0,
			csprit: [[[0u8,.. 3],.. 4],.. 8],
			vrambank: 0,
		}
	}

	pub fn new_cgb() -> GPU {
		GPU::new()
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
			0x8000 .. 0x9FFF => self.vram[(self.vrambank * 0x2000) | (a & 0x1FFF)],
			0xFE00 .. 0xFE9F => self.voam[a - 0xFE00],
			0xFF40 => {
				(if self.lcd_on { 0x80 } else { 0 }) |
				(if self.win_tilemap == 0x9C00 { 0x40 } else { 0 }) |
				(if self.win_on { 0x20 } else { 0 }) |
				(if self.tilebase == 0x8000 { 0x10 } else { 0 }) |
				(if self.bg_tilemap == 0x9C00 { 0x08 } else { 0 }) |
				(if self.sprite_size == 16 { 0x04 } else { 0 }) |
				(if self.sprite_on { 0x02 } else { 0 }) |
				(if self.lcdc0 { 0x01 } else { 0 })
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
			0xFF4F => self.vrambank as u8,
			0xFF68 => { self.cbgpal_ind | (if self.cbgpal_inc { 0x80 } else { 0 }) },
			0xFF69 => {
				let palnum = self.cbgpal_ind >> 3;
				let colnum = (self.cbgpal_ind >> 1) & 0x3;
				if self.cbgpal_ind & 0x01 == 0x00 {
					self.cbgpal[palnum][colnum][0] | ((self.cbgpal[palnum][colnum][1] & 0x07) << 5)
				} else {
					((self.cbgpal[palnum][colnum][1] & 0x18) >> 3) | (self.cbgpal[palnum][colnum][2] << 2)
				}
			},
			0xFF6A => { self.csprit_ind | (if self.csprit_inc { 0x80 } else { 0 }) },
			0xFF6B => {
				let palnum = self.csprit_ind >> 3;
				let colnum = (self.csprit_ind >> 1) & 0x3;
				if self.csprit_ind & 0x01 == 0x00 {
					self.csprit[palnum][colnum][0] | ((self.csprit[palnum][colnum][1] & 0x07) << 5)
				} else {
					((self.csprit[palnum][colnum][1] & 0x18) >> 3) | (self.csprit[palnum][colnum][2] << 2)
				}
			},
			_ => fail!("GPU does not handle read {:04X}", a),
		}
	}

	fn rbvram0(&self, a: u16) -> u8 {
		self.vram[a & 0x1FFF]
	}
	fn rbvram1(&self, a: u16) -> u8 {
		self.vram[0x2000 + (a & 0x1FFF)]
	}

	pub fn wb(&mut self, a: u16, v: u8) {
		match a {
			0x8000 .. 0x9FFF => self.vram[(self.vrambank * 0x2000) | (a & 0x1FFF)] = v,
			0xFE00 .. 0xFE9F => self.voam[a - 0xFE00] = v,
			0xFF40 => {
				let orig_lcd_on = self.lcd_on;
				self.lcd_on = v & 0x80 == 0x80;
				self.win_tilemap = if v & 0x40 == 0x40 { 0x9C00 } else { 0x9800 };
				self.win_on = v & 0x20 == 0x20;
				self.tilebase = if v & 0x10 == 0x10 { 0x8000 } else { 0x8800 };
				self.bg_tilemap = if v & 0x08 == 0x08 { 0x9C00 } else { 0x9800 };
				self.sprite_size = if v & 0x04 == 0x04 { 16 } else { 8 };
				self.sprite_on = v & 0x02 == 0x02;
				self.lcdc0 = v & 0x01 == 0x01;
				if orig_lcd_on && !self.lcd_on { self.modeclock = 0; self.line = 0; self.mode = 0; }
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
			0xFF4F => self.vrambank = (v & 0x01) as u16,
			0xFF68 => { self.cbgpal_ind = v & 0x3F; self.cbgpal_inc = v & 0x80 == 0x80; },
			0xFF69 => {
				let palnum = self.cbgpal_ind >> 3;
				let colnum = (self.cbgpal_ind >> 1) & 0x03;
				if self.cbgpal_ind & 0x01 == 0x00 {
					self.cbgpal[palnum][colnum][0] = v & 0x1F;
					self.cbgpal[palnum][colnum][1] = (self.cbgpal[palnum][colnum][1] & 0x18) | (v >> 5);
				} else {
					self.cbgpal[palnum][colnum][1] = (self.cbgpal[palnum][colnum][1] & 0x07) | ((v & 0x3) << 3);
					self.cbgpal[palnum][colnum][2] = (v >> 2) & 0x1F;
				}
				if self.cbgpal_inc { self.cbgpal_ind = (self.cbgpal_ind + 1) & 0x3F; };
			},
			0xFF6A => { self.csprit_ind = v & 0x3F; self.csprit_inc = v & 0x80 == 0x80; },
			0xFF6B => {
				let palnum = self.csprit_ind >> 3;
				let colnum = (self.csprit_ind >> 1) & 0x03;
				if self.csprit_ind & 0x01 == 0x00 {
					self.csprit[palnum][colnum][0] = v & 0x1F;
					self.csprit[palnum][colnum][1] = (self.csprit[palnum][colnum][1] & 0x18) | (v >> 5);
				} else {
					self.csprit[palnum][colnum][1] = (self.csprit[palnum][colnum][1] & 0x07) | ((v & 0x3) << 3);
					self.csprit[palnum][colnum][2] = (v >> 2) & 0x1F;
				}
				if self.csprit_inc { self.csprit_ind = (self.csprit_ind + 1) & 0x3F; };
			},
			_ => fail!("GPU does not handle write {:04X}", a),
		}
	}

	fn update_pal(&mut self) {
		for i in range(0, 4) {
			self.palb[i] = GPU::get_monochrome_pal_val(self.palbr, i);
			self.pal0[i] = GPU::get_monochrome_pal_val(self.pal0r, i);
			self.pal1[i] = GPU::get_monochrome_pal_val(self.pal1r, i);
		}
	}
	fn get_monochrome_pal_val(value: u8, index: int) -> u8 {
		match (value >> 2*index) & 0x03 {
			0 => 255,
			1 => 192,
			2 => 96,
			_ => 0
		}
	}

	fn renderscan(&mut self) {
		for x in range(0, SCREEN_W) {
			self.setcolor(x, 255);
			self.bgprio[x] = Normal;
		}
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
		if self.gbmode != ::gbmode::Color && !self.lcdc0 { return }

		let bgy = self.scy + self.line;
		let tiley = (bgy as u16 >> 3) & 31;
		for x in range(0, SCREEN_W) {
			let bgx = self.scx as uint + x;
			let tilex = (bgx as u16 >> 3) & 31;

			let tilenr: u8 = self.rbvram0(self.bg_tilemap + tiley * 32 + tilex);

			let (palnr, vram0, xflip, yflip, prio) = if self.gbmode == ::gbmode::Color {
				let flags = self.rbvram1(self.bg_tilemap + tiley * 32 + tilex);
				(flags & 0x07,
				flags & (1 << 3) == 0,
				flags & (1 << 5) != 0,
				flags & (1 << 6) != 0,
				flags & (1 << 7) != 0)
			} else {
				(0, true, false, false, false)
			};

			let tileaddress = (self.tilebase
			+ (if self.tilebase == 0x8000 {
				tilenr as u16
			} else {
				(tilenr as i8 as i16 + 128) as u16
			}) * 16);

			let a0 = match yflip {
				false => tileaddress + ((bgy as u16 & 0x07) * 2),
				true => tileaddress + (14 - ((bgy as u16 & 0x07) * 2)),
			};

			let (b1, b2) = match vram0 {
				true => (self.rbvram0(a0), self.rbvram0(a0 + 1)),
				false => (self.rbvram1(a0), self.rbvram1(a0 + 1)),
			};

			let xbit = match xflip {
				true => bgx & 0x07,
				false => 7 - (bgx & 0x07),
			};
			let colnr = if b1 & (1 << xbit) != 0 { 1 } else { 0 }
				| if b2 & (1 << xbit) != 0 { 2 } else { 0 };

			self.bgprio[x] =
				if prio { PrioFlag }
				else if colnr == 0 { Color0 }
				else { Normal };
			if self.gbmode == ::gbmode::Color {
				let data_a = self.line as uint * SCREEN_W * 3 + x * 3;
				self.data[data_a + 0] = self.cbgpal[palnr][colnr][0] * 8 + 7;
				self.data[data_a + 1] = self.cbgpal[palnr][colnr][1] * 8 + 7;
				self.data[data_a + 2] = self.cbgpal[palnr][colnr][2] * 8 + 7;
			} else {
				self.setcolor(x, self.palb[colnr]);
			}
		}
	}

	fn draw_win(&mut self) {
		if !self.win_on || (self.gbmode != ::gbmode::Classic && !self.lcdc0) {
			return
		}

		let winy = self.line as int - self.winy as int;
		if winy < 0 { return }

		let tiley: u16 = ((winy as u16) >> 3) & 31;

		for x in range(0, SCREEN_W) {
			let winx: int = - ((self.winx as int) - 7) + (x as int);
			if winx < 0 { continue }
			let tilex: u16 = ((winx as u16) >> 3) & 31;

			let tilenr: u8 = self.rbvram0(self.win_tilemap + tiley * 32 + tilex);

			let (palnr, vram1, xflip, yflip, prio) = if self.gbmode == ::gbmode::Color {
				let flags = self.rbvram1(self.win_tilemap + tiley * 32 + tilex);
				(flags & 0x07,
				flags & (1 << 3) != 0,
				flags & (1 << 5) != 0,
				flags & (1 << 6) != 0,
				flags & (1 << 7) != 0)
			} else {
				(0, false, false, false, false)
			};

			let tileaddress = (self.tilebase
			+ (if self.tilebase == 0x8000 {
				tilenr as u16
			} else {
				(tilenr as i8 as i16 + 128) as u16
			}) * 16);

			let a0 = match yflip {
				false => tileaddress + ((winy as u16 & 0x07) * 2),
				true  => tileaddress + (14 - ((winy as u16 & 0x07) * 2)),
			};

			let (b1, b2) = match vram1 {
				false => (self.rbvram0(a0), self.rbvram0(a0 + 1)),
				true  => (self.rbvram1(a0), self.rbvram1(a0 + 1)),
			};

			let xbit = match xflip {
				true  => winx & 0x07,
				false => 7 - (winx & 0x07),
			};

			let colnr = if b1 & (1 << xbit) != 0 { 1 } else { 0 }
				| if b2 & (1 << xbit) != 0 { 2 } else { 0 };

			self.bgprio[x] =
				if prio { PrioFlag }
				else if colnr == 0 { Color0 }
				else { Normal };
			if self.gbmode == ::gbmode::Color {
				let data_a = self.line as uint * SCREEN_W * 3 + x * 3;
				self.data[data_a + 0] = self.cbgpal[palnr][colnr][0] * 8 + 7;
				self.data[data_a + 1] = self.cbgpal[palnr][colnr][1] * 8 + 7;
				self.data[data_a + 2] = self.cbgpal[palnr][colnr][2] * 8 + 7;
			} else {
				self.setcolor(x, self.palb[colnr]);
			}
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
			let c_palnr: u8 = flags & 0x07;
			let c_vram0: bool = flags & (1 << 3) == 0;

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
			let (b1, b2) = if !c_vram0 && self.gbmode == ::gbmode::Color {
				(self.rbvram1(tileaddress), self.rbvram1(tileaddress + 1))
			} else {
				(self.rbvram0(tileaddress), self.rbvram0(tileaddress + 1))
			};

			'xloop: for x in range(0, 8) {
				if spritex + x < 0 || spritex + x >= (SCREEN_W as int) { continue }

				let xbit: u8 = 1 << (if xflip { x } else { 7 - x });
				let colnr: u8 = (if b1 & xbit != 0 { 1 } else { 0 }) |
					(if b2 & xbit != 0 { 2 } else { 0 });
				if colnr == 0 { continue }

				if self.gbmode == ::gbmode::Color {
					if self.lcdc0 && (self.bgprio[spritex + x] == PrioFlag || (belowbg && self.bgprio[spritex + x] != Color0)) {
						continue 'xloop
					}
					let data_a = self.line as uint * SCREEN_W * 3 + ((spritex + x) as uint) * 3;
					self.data[data_a + 0] = self.csprit[c_palnr][colnr][0] * 8 + 7;
					self.data[data_a + 1] = self.csprit[c_palnr][colnr][1] * 8 + 7;
					self.data[data_a + 2] = self.csprit[c_palnr][colnr][2] * 8 + 7;
				} else {
					if belowbg && self.bgprio[spritex + x] != Color0 { continue 'xloop }
					let color = if usepal1 { self.pal1[colnr] } else { self.pal0[colnr] };
					self.setcolor((spritex + x) as uint, color);
				}
			}
		}
	}
}
