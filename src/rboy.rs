#![crate_name = "rboy"]
#![crate_type = "lib" ]

#![feature(old_io, io, core, path, fs)]
#![cfg_attr(test, feature(test))]

#[macro_use]
extern crate log;

extern crate time;

#[cfg(test)]
extern crate test;

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
mod util;
