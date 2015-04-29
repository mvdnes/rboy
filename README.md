RBoy
====

A Gameboy Color Emulator written in Rust using SDL

Build status
------------
[![Build Status](https://travis-ci.org/mvdnes/rboy.png?branch=master)](https://travis-ci.org/mvdnes/rboy)
[![Build Status](https://api.shippable.com/projects/553fdfb4edd7f2c052d66b4e/badge?branchName=master)](https://app.shippable.com/projects/553fdfb4edd7f2c052d66b4e/builds/latest)

Dependencies
------------

* Rust (master)
* SDL

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
