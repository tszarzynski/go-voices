# GO Voices. `just --list --list-submodules` shows every recipe.
#
#   just               check + test (run before committing)
#   just apple build   iPhone/iPad and Mac app (apple/Justfile)

mod apple

default: check test

# format Rust code
fix:
    cargo fmt

# formatting and lints (pedantic/nursery clippy, warnings are errors)
check:
    cargo fmt -- --check
    cargo clippy --workspace --all-targets --features shared_ffi/codegen -- -D warnings

# Rust unit and Crux app tests
test:
    cargo nextest run --workspace

# the core builds without std
check-nostd:
    cargo build -p catalogue -p crux-midi -p shared --no-default-features --features crux_core/critical-section --target thumbv7em-none-eabihf

# rebuild the catalogue from GO:Plus's list of sounds
import-sounds source="../goplus/GO-sounds.md":
    cargo run -p import-sounds -- {{ source }} catalogue/data/patches.tsv
