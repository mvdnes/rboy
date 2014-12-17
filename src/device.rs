use cpu::CPU;
use keypad::KeypadKey;

pub struct Device
{
	cpu: CPU<'static>,
}

fn stdoutprinter(v: u8) -> u8
{
	print!("{}", v as char);
	::std::io::stdio::flush();
	0
}

impl Device
{
	pub fn new(romname: &str) -> Option<Device>
	{
		match CPU::new(romname, None)
		{
			Some(cpu) => Some(Device { cpu: cpu }),
			None => None,
		}
	}

	pub fn new_cgb(romname: &str) -> Option<Device>
	{
		match CPU::new_cgb(romname, None)
		{
			Some(cpu) => Some(Device { cpu: cpu }),
			None => None,
		}
	}

	pub fn do_cycle(&mut self) -> uint
	{
		self.cpu.do_cycle()
	}

	pub fn set_stdout(&mut self, output: bool)
	{
		if output {
			self.cpu.mmu.serial.set_callback(stdoutprinter);
		} else {
			self.cpu.mmu.serial.unset_callback();
		}
	}

	pub fn check_and_reset_gpu_updated(&mut self) -> bool
	{
		let result = self.cpu.mmu.gpu.updated;
		self.cpu.mmu.gpu.updated = false;
		result
	}

	pub fn get_gpu_data<'a>(&'a self) -> &'a [u8, ..160 * 144 * 3]
	{
		&self.cpu.mmu.gpu.data
	}

	pub fn keyup(&mut self, key: KeypadKey)
	{
		self.cpu.mmu.keypad.keyup(key);
	}

	pub fn keydown(&mut self, key: KeypadKey)
	{
		self.cpu.mmu.keypad.keydown(key);
	}
}
