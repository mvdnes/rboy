.PHONY: src
src: lib/libsdl-0.3.1.so
	cd src && make

lib:
	mkdir lib

rust-sdl/README.md:
	git submodule update --init rust-sdl

lib/libsdl-0.3.1.so: lib rust-sdl/README.md
	cd rust-sdl && rustpkg build sdl
	mv rust-sdl/build/*/sdl/libsdl* lib/libsdl-0.3.1.so
	rm -rf rust-sdl/buid rust-sdl/.rust

.PHONY: clean
clean:
	cd src && make clean

.PHONY: distclean
distclean: clean
	git submodule deinit -f rust-sdl
	rm -rf lib
