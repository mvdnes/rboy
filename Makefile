src/tester: src/*.rs lib/libsdl-0.3.1.so
	rustc -O -A dead_code -L lib/ src/tester.rs

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
	rm -f src/tester

.PHONY: distclean
distclean: clean
	git submodule deinit -f rust-sdl
	rm -rf lib
