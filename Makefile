.PHONY: src
src: lib/libsdl-0.3.1.rlib
	cd src && make

rust-sdl/README.md: .gitmodules
	git submodule sync rust-sdl
	git submodule update --init rust-sdl

lib/libsdl-0.3.1.rlib: rust-sdl/README.md
	cd rust-sdl && rustc src/sdl/lib.rs
	mkdir -p lib
	mv rust-sdl/libsdl*.rlib lib/libsdl-0.3.1.rlib

.PHONY: clean
clean:
	cd src && make clean

.PHONY: distclean
distclean: clean
	git submodule deinit -f rust-sdl
	rm -rf lib
