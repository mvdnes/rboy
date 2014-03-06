rboy
====

A Gameboy Emulator in Rust using SDL

Build status
------------
[![Build Status](https://travis-ci.org/mvdnes/rboy.png?branch=master)](https://travis-ci.org/mvdnes/rboy)

Dependencies
------------

* Rust 0.10-pre
* SDL

Implemented
-----------

* CPU
  - All instructions correct
  - All timings correct
* GPU
* Keypad
* Timer
* MMU
  - MBC-less
  - MBC1
  - MBC3 (with RTC)
  - MBC5
  - save games


Special thanks to
-----------------

* http://imrannazar.com/GameBoy-Emulation-in-JavaScript:-The-CPU
* http://nocash.emubase.de/pandocs.htm
* https://github.com/alexcrichton/jba (The Rust branch)
