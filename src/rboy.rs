#![crate_name = "rboy"]
#![crate_type = "lib" ]

#![feature(convert)]

extern crate time;
extern crate podio;

pub use keypad::KeypadKey;

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
