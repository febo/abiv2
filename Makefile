RUST_TOOLCHAIN_NIGHTLY = nightly-2026-01-22

nightly = +${RUST_TOOLCHAIN_NIGHTLY}
# Convert 'programs/anything' to 'anything'.
program-target = $(subst /,-,$(patsubst programs/%,%,$1))
# All files directly inside programs.
PROGRAMS := $(wildcard programs/*)
# Generate the dashed target program names.
PROGRAM_TARGETS := $(foreach src,$(PROGRAMS),$(call program-target,$(src)))

# Run `cargo bench`.
bench:
	@cargo bench $(ARGS)

# Build all programs.
all:
	@for dir in $(PROGRAM_TARGETS); do \
		$(MAKE) build-$$dir; \
	done

# Build a program.
build-%:
	@RUSTFLAGS="-C embed-bitcode=yes -C lto=fat" cargo build-sbf --manifest-path programs/$*/Cargo.toml --arch v3 --abi-v2 $(ARGS)

# Run `cargo clean`.
clean:
	@cargo clean

# Run `cargo clippy`.
clippy:
	@cargo clippy \
		--workspace --all-targets -- \
		--deny=warnings \
		--deny=clippy::default_trait_access \
		--deny=clippy::arithmetic_side_effects \
		--deny=clippy::manual_let_else \
		--deny=clippy::used_underscore_binding

# Run `cargo fmt`.
format:
	@cargo $(nightly) fmt --all $(ARGS)

test:
	SBF_OUT_DIR=$(PWD)/target/deploy cargo test --manifest-path benchmark/Cargo.toml $(ARGS)
