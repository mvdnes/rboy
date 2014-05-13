RUSTC?=rustc

.PHONY: src
src: rust-sdl-build
	cd src && make

.PHONY: test
test: rust-sdl-build
	cd src && make test

rust-sdl/README.md: .gitmodules
	git submodule sync rust-sdl
	git submodule update --init rust-sdl

rust-sdl-build: rust-sdl/README.md
	cd rust-sdl && $(RUSTC) -O src/sdl/lib.rs
	mkdir -p lib
	mv rust-sdl/libsdl*.rlib lib/
	touch rust-sdl-build

.PHONY: clean
clean:
	cd src && make clean

.PHONY: distclean
distclean: clean
	git submodule deinit -f rust-sdl
	rm -rf lib rust-sdl-build
