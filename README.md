# RBoy

A Gameboy Color Emulator written in Rust


## QuickStart

You can clone this repository and build it using either the `make` command or
`cargo build --release`. Running the emulator can be done either via `cargo run --release`, or by
running the generated binary found in `target/release`. You can copy the executable named `rboy` or
`rboy.exe` to some sort of binary directory such as `~/.local/bin/` in linux or something under the
`PATH` in windows.

Then you can explore the ability of the emulator by `rboy --help`. Which outputs 

```
A Gameboy Colour emulator written in Rust

Usage: rboy [OPTIONS] <filename>

Arguments:
  <filename>  Sets the ROM file to load

Options:
  -s, --serial         Prints the data from the serial port to stdout
  -p, --printer        Emulates a gameboy printer
  -c, --classic        Forces the emulator to run in classic Gameboy mode
  -x, --scale <scale>  Sets the scale of the interface. Default: 2
  -a, --audio          Enables audio
      --skip-checksum  Skips verification of the cartridge checksum
      --test-mode      Starts the emulator in a special test mode
  -h, --help           Print help
  -V, --version        Print version
```

Now you can look below for the Keybindings section below.

## Keybindings

### Gameplay Keybindings

| Key on Keyboard    | Emulator Key       |
| ------------------ | ------------------ |
| Z                  | A                  |
| X                  | B                  |
| Up/Down/Left/Right | Up/Down/Left/Right |
| Space              | Select             |
| Return/Enter       | Start              |

### General Keybindings

| Key on Keyboard   | Emulator Action                     |
| ----------------- | ----------------------------------- |
| 1                 | Switch to 1:1 scale                 |
| R                 | Restore scale given on command line |
| Left Shift (Hold) | Unrestricted Speed Mode             |
| T                 | Change pixel interpolation          |

## Implemented


* CPU
  - All instructions correct
  - All timings correct
  - Double speed mode
* GPU
  - Normal mode
  - Color mode
* Keypad
* Timer
* Audio
* MMU
  - MBC-less
  - MBC1
  - MBC3 (with RTC)
  - MBC5
  - save games
* Printing

## Test mode
The test mode, activated with the `--test-mode` flag, provides some functionality for running
[GBEmulatorShootout](https://github.com/daid/GBEmulatorShootout). This is still under development.

## Special thanks to

* http://imrannazar.com/GameBoy-Emulation-in-JavaScript:-The-CPU
* http://bgb.bircd.org/pandocs.htm
* https://github.com/alexcrichton/jba (The Rust branch)
* https://gbdev.io/pandocs/
