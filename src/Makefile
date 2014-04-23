RUSTC?=rustc
PACKEDROMS=$(wildcard tests/*.gb.gz)
ROMS=$(PACKEDROMS:.gb.gz=.gb)

rboy: *.rs
	$(RUSTC) -O -L ../lib/ main.rs

rboy_test: *.rs
	$(RUSTC) -O -L ../lib/ main.rs --test -A dead_code -o $@

.PHONY: clean
clean:
	$(RM) rboy rboy_test $(ROMS)

.PHONY: test
test: rboy_test $(ROMS)
	./rboy_test

$(ROMS): %.gb : %.gb.gz
	gunzip -c $< > $@
