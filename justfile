# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
# MeedyaManager — Task runner (https://github.com/casey/just)
# Install: cargo install just

# Default recipe — show available commands
default:
    @just --list

# Build all Rust crates (excludes GTK4 which needs Linux system libs)
build:
    cargo build --workspace

# Build in release mode
build-release:
    cargo build --workspace --release

# Run all tests
test:
    cargo test --workspace

# Run clippy linter
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Apply formatting
fmt:
    cargo fmt --all

# Run all checks (format + lint + test)
check: fmt-check lint test

# Build the CLI binary
cli:
    cargo build -p mm-cli

# Run the CLI
run *ARGS:
    cargo run -p mm-cli -- {{ARGS}}

# Build macOS app (requires macOS + Swift toolchain)
macos:
    cd macos && swift build

# Build Windows app (requires Windows + .NET 8 SDK)
windows:
    cd windows && dotnet build

# Build Linux GTK4 app (requires libgtk-4-dev + libadwaita-1-dev)
linux:
    cargo build -p mm-gtk

# Generate API documentation
docs:
    cargo doc --workspace --no-deps --open

# Run security audit
audit:
    cargo deny check
    cargo audit

# Display current workspace version
version:
    @grep -A 20 '\[workspace\.package\]' Cargo.toml | grep '^version' | head -1 | sed 's/.*"\(.*\)"/\1/'

# Build release artifacts locally for testing
release-local:
    cargo build --workspace --release
    @echo "Release binaries in target/release/"

# Clean all build artifacts
clean:
    cargo clean
    rm -rf macos/.build
