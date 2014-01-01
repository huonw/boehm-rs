include rust.mk
RUSTC ?= rustc
RUSTFLAGS ?= -O

.PHONY: all examples

all: boehm examples
examples: example lowlevel_example

$(eval $(call RUST_CRATE, .))

lowlevel_example: lowlevel_example.rs $(_rust_crate_lib)
	$(RUSTC) -L. lowlevel_example.rs -o lowlevel_example

example:  example.rs $(_rust_crate_lib)
	$(RUSTC) -L. example.rs -o example
