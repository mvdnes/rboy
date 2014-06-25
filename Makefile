RUSTC?=rustc

SOURCES=$(wildcard src/*.rs) $(wildcard src/*/*.rs)
PACKEDROMS=$(wildcard roms/*.gb.gz)
ROMS=$(PACKEDROMS:.gb.gz=.gb)

rboy: rust-sdl-build $(SOURCES)
	$(RUSTC) -O -L lib src/main.rs

rboy_test: rust-sdl-build $(SOURCES)
	$(RUSTC) -O -L lib src/main.rs --test -A dead_code -o $@

.PHONY: test
test: rboy_test $(ROMS)
	./rboy_test

$(ROMS): %.gb : %.gb.gz
	gunzip -c $< > $@

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
	$(RM) rboy rboy_test $(ROMS)

.PHONY: distclean
distclean: clean
	git submodule deinit -f rust-sdl
	$(RM) -rf lib rust-sdl-build
