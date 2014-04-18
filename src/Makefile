RUSTC?=rustc

rboy: *.rs
	$(RUSTC) -O -L ../lib/ main.rs

rboy_test: *.rs
	$(RUSTC) -O -L ../lib/ main.rs --test -A dead_code -o $@

.PHONY: test
test: rboy_test
	./rboy_test

.PHONY: clean
clean:
	$(RM) rboy rboy_test
