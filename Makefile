RUSTC?=rustc
CARGO?=cargo

SOURCES=$(wildcard src/*.rs) $(wildcard src/*/*.rs)
PACKEDROMS=$(wildcard roms/*.gb.gz)
ROMS=$(PACKEDROMS:.gb.gz=.gb)
TARGET=target
OPT_TARGET=opt

$(TARGET)/rboy: $(SOURCES)
	$(CARGO) build

.PHONY: opt
opt: $(TARGET)/rboy
	mkdir -p $(OPT_TARGET)
	$(RUSTC) -O src/rboy.rs --out-dir $(OPT_TARGET) -L $(TARGET)/deps
	$(RUSTC) -L $(OPT_TARGET) -L $(TARGET)/deps -O src/bin/rboy.rs --out-dir $(OPT_TARGET)

.PHONY: test
test: $(ROMS)
	$(CARGO) test

$(ROMS): %.gb : %.gb.gz
	gunzip -c $< > $@

.PHONY: clean
clean:
	$(CARGO) clean
	$(RM) -r $(OPT_TARGET) $(ROMS)
