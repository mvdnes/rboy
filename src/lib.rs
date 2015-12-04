#![crate_name = "rboy"]
#![crate_type = "lib" ]

extern crate blip_buf;
extern crate podio;
extern crate time;

pub use keypad::KeypadKey;
pub use gpu::{SCREEN_W, SCREEN_H};
pub use sound::AudioPlayer;

pub mod device;

mod cpu;
mod gbmode;
mod gpu;
mod keypad;
mod mbc;
mod mmu;
mod register;
mod serial;
mod sound;
mod timer;

pub type StrResult<T> = Result<T, &'static str>;
