use util::handle_io;

mod mbc0;
mod mbc1;
mod mbc3;
mod mbc5;

pub trait MBC {
	fn readrom(&self, a: u16) -> u8;
	fn readram(&self, a: u16) -> u8;
	fn writerom(&mut self, a: u16, v: u8);
	fn writeram(&mut self, a: u16, v: u8);
}

pub fn get_mbc(file: &Path) -> Option<Box<MBC+'static>> {
	let data: Vec<u8> = match handle_io(::std::io::File::open(file).read_to_end(), "Could not read ROM")
	{
		Some(mbc) => { mbc },
		None => { return None; },
	};
	if data.len() < 0x150 { error!("Rom size to small"); return None; }
	if !check_checksum(&data)
	{
		return None;
	}
	match data[0x147] {
		0x00 => mbc0::MBC0::new(data).map(|v| box v as Box<MBC>),
		0x01 ... 0x03 => mbc1::MBC1::new(data, file).map(|v| box v as Box<MBC>),
		0x0F ... 0x13 => mbc3::MBC3::new(data, file).map(|v| box v as Box<MBC>),
		0x19 ... 0x1E => mbc5::MBC5::new(data, file).map(|v| box v as Box<MBC>),
		m => { error!("Unsupported MBC type: {:02X}", m); None },
	}
}

fn ram_size(v: u8) -> uint {
	match v {
		1 => 0x800,
		2 => 0x2000,
		3 => 0x8000,
		4 => 0x20000,
		_ => 0,
	}
}

fn check_checksum(data: &Vec<u8>) -> bool {
	let mut value: u8 = 0;
	for i in range(0x134u, 0x14D) {
		value = value - data[i] - 1;
	}
	match data[0x14D] == value
	{
		true => true,
		false => { error!("Cartridge checksum is invalid. {:02X} != {:02X}", data[0x14D], value); false },
	}
}
