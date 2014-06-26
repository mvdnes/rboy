RUSTC?=rustc
CARGO?=cargo

SOURCES=$(wildcard src/*.rs) $(wildcard src/*/*.rs)
PACKEDROMS=$(wildcard roms/*.gb.gz)
ROMS=$(PACKEDROMS:.gb.gz=.gb)

target/rboy: $(SOURCES)
	$(CARGO) build

target/rboy_test: $(SOURCES)
	$(RUSTC) -O -L target/deps src/rboy.rs --test -A dead_code -o $@

.PHONY: test
test: target/rboy_test $(ROMS)
	$<

$(ROMS): %.gb : %.gb.gz
	gunzip -c $< > $@

.PHONY: clean
clean:
	$(RM) target/rboy target/rboy_test $(ROMS)

.PHONY: distclean
distclean: clean
	$(RM) -r target
