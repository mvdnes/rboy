RUSTC?=rustc
CARGO?=cargo

SOURCES=$(wildcard src/*.rs) $(wildcard src/*/*.rs)
PACKEDROMS=$(wildcard roms/*.gb.gz)
ROMS=$(PACKEDROMS:.gb.gz=.gb)
TARGET=target

$(TARGET)/rboy: $(SOURCES)
	$(CARGO) build

$(TARGET)/rboy_test: $(SOURCES)
	mkdir -p $(TARGET)
	$(RUSTC) -O src/rboy.rs --test -o $@

.PHONY: test
test: $(TARGET)/rboy_test $(ROMS)
	$<

$(ROMS): %.gb : %.gb.gz
	gunzip -c $< > $@

.PHONY: clean
clean:
	$(RM) $(TARGET)/rboy $(TARGET)/rboy_test $(ROMS)

.PHONY: distclean
distclean: clean
	$(RM) -r $(TARGET)
