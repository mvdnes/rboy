CARGO?=cargo

PACKEDROMS=$(wildcard roms/*.gb.gz)
ROMS=$(PACKEDROMS:.gb.gz=.gb)

.PHONY: release
release:
	$(CARGO) build --release --features=gui

.PHONY: debug
debug:
	$(CARGO) build --features=gui

.PHONY: test
test: $(ROMS)
	$(CARGO) test

$(ROMS): %.gb : %.gb.gz
	gunzip -c $< > $@

.PHONY: clean
clean:
	$(CARGO) clean
	$(RM) -r $(ROMS)
