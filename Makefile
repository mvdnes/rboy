RUSTC?=rustc
CARGO?=cargo

SOURCES=$(wildcard src/*.rs) $(wildcard src/*/*.rs)
PACKEDROMS=$(wildcard roms/*.gb.gz)
ROMS=$(PACKEDROMS:.gb.gz=.gb)
TARGET=target

.PHONY: all
all: $(TARGET)/rboy

$(TARGET)/rboy: $(SOURCES)
	$(CARGO) build

.PHONY: opt
opt: $(TARGET)/release/rboy

$(TARGET)/release/rboy:
	$(CARGO) build --release

.PHONY: test
test: $(ROMS)
	$(CARGO) test

$(ROMS): %.gb : %.gb.gz
	gunzip -c $< > $@

.PHONY: clean
clean:
	$(CARGO) clean
	$(RM) -r $(ROMS)
