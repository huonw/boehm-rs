include rust.mk
RUSTC ?= rustc
RUSTFLAGS ?= -O
EG=examples

.PHONY: all examples

all: boehm examples
examples: example lowlevel_example tracing_example tracing_example_conservative

$(eval $(call RUST_CRATE, ./src/))

lowlevel_example: $(EG)/lowlevel_example.rs $(_rust_crate_lib)
	$(RUSTC) $(RUSTFLAGS) -L. $(EG)/lowlevel_example.rs -o lowlevel_example

example:  $(EG)/example.rs $(_rust_crate_lib)
	$(RUSTC) $(RUSTFLAGS) -L. $(EG)/example.rs -o example

tracing_example: $(EG)/tracing_example.rs $(_rust_crate_lib)
	$(RUSTC) $(RUSTFLAGS) -L. $(EG)/tracing_example.rs -o tracing_example

tracing_example_conservative: $(EG)/tracing_example.rs $(_rust_crate_lib)
	$(RUSTC) $(RUSTFLAGS) -L. $(EG)/tracing_example.rs -o tracing_example_conservative --cfg conservative
