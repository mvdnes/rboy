#![crate_id = "rboy"]
#![license = "MIT"]
#![crate_type = "lib"]

#![feature(phase)]
#[phase(plugin, link)] extern crate log;

extern crate time;
#[cfg(test)]
extern crate test;

pub use keypad::{KeypadKey, Right, Left, Up, Down, A, B, Select, Start};

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
mod util;
