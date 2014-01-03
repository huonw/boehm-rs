include rust.mk
RUSTC ?= rustc
RUSTFLAGS ?= -O

.PHONY: all examples

all: boehm examples
examples: example lowlevel_example tracing_example

$(eval $(call RUST_CRATE, .))

lowlevel_example: lowlevel_example.rs $(_rust_crate_lib)
	$(RUSTC) $(RUSTFLAGS) -L. lowlevel_example.rs -o lowlevel_example

example:  example.rs $(_rust_crate_lib)
	$(RUSTC) $(RUSTFLAGS) -L. example.rs -o example

tracing_example: tracing_example.rs $(_rust_crate_lib)
	$(RUSTC) $(RUSTFLAGS) -L. tracing_example.rs -o tracing_example
