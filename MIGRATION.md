# Migration Guide: Moving sparkplug-rs to a Separate Repository

This document explains how to move the `sparkplug-rs` Rust bindings to its own repository.

## Overview

The `sparkplug-rs` directory is now fully standalone and can be moved to a separate Git repository. The `build.rs` script automatically:
- Clones the C++ library from https://github.com/jsulmont/spark-plug_cpp.git
- Builds the `sparkplug_c` shared library using CMake
- Generates Rust FFI bindings
- Links everything together

Users only need to run `cargo build` - no manual C++ library setup required!

## Prerequisites for New Repository

The build process requires:
- Rust 1.70+ (2021 edition)
- C++23 compiler (Clang 16+ or GCC 13+)
- CMake 3.25+
- Git
- System dependencies (Eclipse Paho MQTT C, Protocol Buffers, Abseil)

## Steps to Create New Repository

### 1. Create the new repository on GitHub

```bash
# On GitHub, create a new repository: sparkplug-rs
# Do NOT initialize with README (we'll push our existing code)
```

### 2. Move the directory and initialize git

```bash
# From outside sparkplug_cpp directory
cp -r sparkplug_cpp/sparkplug-rs ~/sparkplug-rs
cd ~/sparkplug-rs

# Initialize as new git repository
git init
git add .
git commit -m "feat: initial commit of standalone Rust bindings

Idiomatic Rust wrapper for Sparkplug B 2.2 protocol using FFI bindings
to the C++ sparkplug_cpp library.

Features:
- Thread-safe Publisher and Subscriber
- Automatic C++ library fetch and build via build.rs
- Type-safe payload building with builder pattern
- Comprehensive examples
- Zero-copy FFI where possible"

# Add remote and push
git remote add origin https://github.com/jsulmont/sparkplug-rs.git
git branch -M main
git push -u origin main
```

### 3. Test the standalone build

```bash
# Clean build from scratch
cargo clean
cargo build --release

# This should:
# 1. Clone spark-plug_cpp from GitHub
# 2. Build the C library
# 3. Generate bindings
# 4. Build the Rust library
```

### 4. Test examples

```bash
# Terminal 1: Start subscriber
RUST_LOG=info cargo run --release --example subscriber

# Terminal 2: Start publisher
RUST_LOG=info cargo run --release --example publisher
```

## Configuration Options

### Pinning to a Specific C++ Library Version

Edit `build.rs` to use a tagged release instead of `main` branch:

```rust
const CPP_REPO_BRANCH: &str = "v0.1.0";  // Pin to specific tag
```

### Using a Fork

Edit `build.rs` to point to your fork:

```rust
const CPP_REPO_URL: &str = "https://github.com/YOUR_USERNAME/spark-plug_cpp.git";
```

## Publishing to crates.io

Before publishing:

1. Ensure all examples work
2. Run tests: `cargo test`
3. Update README.md with installation instructions
4. Verify Cargo.toml metadata is correct
5. Consider pinning to a stable C++ library version tag

```bash
cargo publish --dry-run
cargo publish
```

## Troubleshooting

### Build failures

If the C++ library fails to build, check:
- CMake is installed and in PATH
- C++23 compiler is available
- System dependencies are installed (see C++ library README)

### Library linking errors at runtime

The shared library must be in the library search path. On macOS:
```bash
export DYLD_LIBRARY_PATH="$DYLD_LIBRARY_PATH:/path/to/libsparkplug_c.dylib"
```

On Linux:
```bash
export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:/path/to/libsparkplug_c.so"
```

For deployment, consider:
- Statically linking the C++ library
- Using rpath
- Bundling the shared library with your application

## Maintenance

When the C++ library is updated:
- Users get updates automatically on next build (if using `main` branch)
- Or bump the version tag in `build.rs` to pull in specific updates
- Test thoroughly after C API changes

## CI/CD Considerations

GitHub Actions example:

```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y cmake clang-16 libc++-16-dev \
            libpaho-mqtt-dev protobuf-compiler libprotobuf-dev \
            libabsl-dev

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Build
        run: cargo build --release

      - name: Test
        run: cargo test
```

Note: The C++ library clone and build happens automatically during the Cargo build process.
