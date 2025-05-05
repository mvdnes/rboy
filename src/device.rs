use crate::cpu::CPU;
use crate::gbmode::GbMode;
use crate::keypad::KeypadKey;
use crate::mbc;
use crate::printer::GbPrinter;
use crate::serial;
use crate::serial::SerialCallback;
use crate::sound;
use crate::StrResult;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Device {
    cpu: CPU,
    save_state: Option<String>,
}

impl Drop for Device {
    fn drop(&mut self) {
        if let Some(path) = &self.save_state {
            let file = std::fs::File::create(path).unwrap();
            ciborium::into_writer(&self.cpu, file).unwrap();
        }
    }
}

pub struct StdoutPrinter;

impl SerialCallback for StdoutPrinter {
    fn call(&mut self, v: u8) -> Option<u8> {
        use std::io::Write;

        print!("{}", v as char);
        let _ = ::std::io::stdout().flush();

        None
    }
}

impl Device {
    pub fn load_state(path: &str) -> Option<Box<Device>> {
        let file = std::fs::File::open(path).ok()?;
        let cpu = ciborium::de::from_reader(file).ok()?;
        Some(Box::new(Device {
            cpu,
            save_state: Some(path.to_string()),
        }))
    }

    pub fn new(
        romname: &str,
        skip_checksum: bool,
        save_state: Option<String>,
    ) -> StrResult<Device> {
        let cart = mbc::FileBackedMBC::new(romname.into(), skip_checksum)?;
        CPU::new(Box::new(cart), None).map(|cpu| Device {
            cpu: cpu,
            save_state,
        })
    }

    pub fn new_cgb(
        romname: &str,
        skip_checksum: bool,
        save_state: Option<String>,
    ) -> StrResult<Device> {
        let cart = mbc::FileBackedMBC::new(romname.into(), skip_checksum)?;
        CPU::new_cgb(Box::new(cart), None).map(|cpu| Device {
            cpu: cpu,
            save_state,
        })
    }

    pub fn new_from_buffer(
        romdata: Vec<u8>,
        skip_checksum: bool,
        save_state: Option<String>,
    ) -> StrResult<Device> {
        let cart = mbc::get_mbc(romdata, skip_checksum)?;
        CPU::new(cart, None).map(|cpu| Device {
            cpu: cpu,
            save_state,
        })
    }

    pub fn new_cgb_from_buffer(
        romdata: Vec<u8>,
        skip_checksum: bool,
        save_state: Option<String>,
    ) -> StrResult<Device> {
        let cart = mbc::get_mbc(romdata, skip_checksum)?;
        CPU::new_cgb(cart, None).map(|cpu| Device {
            cpu: cpu,
            save_state,
        })
    }

    pub fn do_cycle(&mut self) -> u32 {
        self.cpu.do_cycle()
    }

    pub fn set_stdout(&mut self, output: bool) {
        if output {
            self.cpu.mmu.serial.set_callback(Box::new(StdoutPrinter));
        } else {
            self.cpu.mmu.serial.unset_callback();
        }
    }

    pub fn attach_printer(&mut self) {
        let printer = GbPrinter::new();

        self.cpu.mmu.serial.set_callback(Box::new(printer));
    }

    pub fn set_serial_callback(&mut self, cb: Box<dyn serial::SerialCallback>) {
        self.cpu.mmu.serial.set_callback(cb);
    }

    pub fn unset_serial_callback(&mut self) {
        self.cpu.mmu.serial.unset_callback();
    }

    pub fn check_and_reset_gpu_updated(&mut self) -> bool {
        let result = self.cpu.mmu.gpu.updated;
        self.cpu.mmu.gpu.updated = false;
        result
    }

    pub fn get_gpu_data(&self) -> &[u8] {
        &self.cpu.mmu.gpu.data
    }

    pub fn enable_audio(&mut self, player: Box<dyn sound::AudioPlayer>, is_on: bool) {
        match self.cpu.mmu.gbmode {
            GbMode::Classic => {
                self.cpu.mmu.sound = Some(sound::Sound::new_dmg(player));
            }
            GbMode::Color | GbMode::ColorAsClassic => {
                self.cpu.mmu.sound = Some(sound::Sound::new_cgb(player));
            }
        };
        if is_on {
            if let Some(sound) = self.cpu.mmu.sound.as_mut() {
                sound.set_on();
            }
        }
    }

    pub fn sync_audio(&mut self) {
        if let Some(ref mut sound) = self.cpu.mmu.sound {
            sound.sync();
        }
    }

    pub fn keyup(&mut self, key: KeypadKey) {
        self.cpu.mmu.keypad.keyup(key);
    }

    pub fn keydown(&mut self, key: KeypadKey) {
        self.cpu.mmu.keypad.keydown(key);
    }

    pub fn romname(&self) -> String {
        self.cpu.mmu.mbc.romname()
    }

    pub fn loadram(&mut self, ramdata: &[u8]) -> StrResult<()> {
        self.cpu.mmu.mbc.loadram(ramdata)
    }

    pub fn dumpram(&self) -> Vec<u8> {
        self.cpu.mmu.mbc.dumpram()
    }

    pub fn ram_is_battery_backed(&self) -> bool {
        self.cpu.mmu.mbc.is_battery_backed()
    }

    pub fn check_and_reset_ram_updated(&mut self) -> bool {
        self.cpu.mmu.mbc.check_and_reset_ram_updated()
    }
}
