# RBoy

[![Build Status](https://travis-ci.org/mvdnes/rboy.png?branch=master)](https://travis-ci.org/mvdnes/rboy)

A Gameboy Color Emulator written in Rust


## QuickStart

To use this emulator you will need to find out alsa development libraries for your system on linux. And then
you can clone this repository and build it using either the `make` command or `cargo build --release`. The generated
binary should be placed under `target/release`. You can copy the executable named `rboy` or `rboy.exe` to some sort
of binary directory such as `~/.local/bin/` in linux or something under the `PATH` in windows.

Then you can explore the ability of the emulator by `rboy --help`. Which outputs 

```
rboy 0.1
Mathijs van de Nes
A Gameboy Colour emulator written in Rust

USAGE:
    rboy [FLAGS] [OPTIONS] <filename>

FLAGS:
    -a, --audio            Enables audio
    -c, --classic          Forces the emulator to run in classic Gameboy mode
    -h, --help             Prints help information
    -p, --printer          Emulates a gameboy printer
    -s, --serial           Prints the data from the serial port to stdout
        --skip-checksum    Skips verification of the cartridge checksum
    -V, --version          Prints version information

OPTIONS:
    -x, --scale <scale>    Sets the scale of the interface. Default: 2

ARGS:
    <filename>    Sets the ROM file to load
```

Now you can look below for the Keybindings section below.

## Keybindings

### Gameplay Keybindings

| Key on Keyboard | Emulator Key |
| --------------- | ------------ |
| Z               | A            |
| X               | B            |
| Up/Down/Left/Right | Up/Down/Left/Right |
| Space           | Select       |
| Return/Enter    | Start        |

### General Keybindings

| Key on Keyboard | Emulator Action |
| --------------- | --------------- |
| 1               | Switch to 1:1 scale |
| R               | Restore scale given on command line |
| Left Shift (Hold) | Unrestricted Speed Mode |
| T               | Change pixel interpolation |

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

## Special thanks to


* http://imrannazar.com/GameBoy-Emulation-in-JavaScript:-The-CPU
* http://nocash.emubase.de/pandocs.htm (Available at http://bgb.bircd.org/pandocs.htm)
* https://github.com/alexcrichton/jba (The Rust branch)
