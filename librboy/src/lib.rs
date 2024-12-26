#![crate_name = "librboy"]
#![crate_type = "lib" ]

pub use crate::keypad::KeypadKey;
pub use crate::gpu::{SCREEN_W, SCREEN_H};
pub use crate::sound::AudioPlayer;

pub mod device;

mod cpu;
mod gbmode;
mod gpu;
mod keypad;
mod mbc;
mod mmu;
mod printer;
mod register;
mod serial;
mod sound;
mod timer;

pub type StrResult<T> = Result<T, &'static str>;
