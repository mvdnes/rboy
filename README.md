RBoy
====

[![Build Status](https://travis-ci.org/mvdnes/rboy.png?branch=master)](https://travis-ci.org/mvdnes/rboy)

A Gameboy Color Emulator written in Rust


Implemented
-----------

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

Special thanks to
-----------------

* http://imrannazar.com/GameBoy-Emulation-in-JavaScript:-The-CPU
* http://nocash.emubase.de/pandocs.htm
* https://github.com/alexcrichton/jba (The Rust branch)
