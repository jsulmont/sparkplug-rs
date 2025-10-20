# Sparkplug Rust Bindings - Quick Start Guide

This guide will help you get started with the Rust FFI bindings for Sparkplug B 2.2.

## Prerequisites

1. C++ library built with shared library support:
   ```bash
   cd /Users/jan/dev/sparkplug_cpp
   cmake --preset default
   cmake --build build --target sparkplug_c
   ```

   This creates `build/src/libsparkplug_c.dylib` (macOS) or `build/src/libsparkplug_c.so` (Linux).

2. MQTT broker running (e.g., Mosquitto):
   ```bash
   brew services start mosquitto
   ```

## Building the Rust Library

```bash
cd sparkplug-rs
cargo build --release
```

## Running Examples

The examples need to find the shared library at runtime. Set the library path:

### macOS

```bash
export DYLD_LIBRARY_PATH=/Users/jan/dev/sparkplug_cpp/build/src:$DYLD_LIBRARY_PATH
cargo run --example publisher
```

In another terminal:

```bash
export DYLD_LIBRARY_PATH=/Users/jan/dev/sparkplug_cpp/build/src:$DYLD_LIBRARY_PATH
cargo run --example subscriber
```

### Linux

```bash
export LD_LIBRARY_PATH=/Users/jan/dev/sparkplug_cpp/build/src:$LD_LIBRARY_PATH
cargo run --example publisher
cargo run --example subscriber
```

## Using in Your Project

Add to your `Cargo.toml`:

```toml
[dependencies]
sparkplug-rs = { path = "../sparkplug-rs" }
```

Then in your code:

```rust
use sparkplug_rs::{Publisher, PublisherConfig, PayloadBuilder};

fn main() -> Result<(), sparkplug_rs::Error> {
    let config = PublisherConfig::new(
        "tcp://localhost:1883",
        "my_client",
        "GroupID",
        "EdgeNode01"
    );

    let mut publisher = Publisher::new(config)?;
    publisher.connect()?;

    let mut birth = PayloadBuilder::new()?;
    birth.add_double_with_alias("Temperature", 1, 23.5);

    let data = birth.serialize()?;
    publisher.publish_birth(&data)?;

    publisher.disconnect()?;
    Ok(())
}
```

## Key Features

- **Thread-Safe**: All types implement `Send + Sync`
- **RAII**: Automatic resource cleanup
- **Type-Safe**: Result-based error handling
- **Idiomatic**: Iterator support for metrics
- **Zero-Copy**: Efficient FFI bindings where possible

## Testing

To run tests (requires shared library in path):

```bash
export DYLD_LIBRARY_PATH=/Users/jan/dev/sparkplug_cpp/build/src:$DYLD_LIBRARY_PATH
cargo test
```

## Documentation

Generate and view documentation:

```bash
cargo doc --open
```

## Troubleshooting

### Library not found at runtime

If you see "Library not loaded: libsparkplug_c.dylib", ensure you've set the library path:

- macOS: `DYLD_LIBRARY_PATH`
- Linux: `LD_LIBRARY_PATH`

### Build errors

If bindgen fails, ensure you have LLVM/Clang installed:

```bash
brew install llvm  # macOS
```

## Architecture Overview

```
sparkplug-rs/
├── src/
│   ├── lib.rs           # Main entry point
│   ├── sys.rs           # Raw FFI bindings (auto-generated)
│   ├── error.rs         # Error types
│   ├── types.rs         # Common types (DataType, Metric, etc.)
│   ├── publisher.rs     # Publisher wrapper
│   ├── subscriber.rs    # Subscriber wrapper
│   └── payload.rs       # PayloadBuilder and Payload parser
├── examples/
│   ├── publisher.rs     # Publisher example
│   └── subscriber.rs    # Subscriber example
├── build.rs             # Build script (runs bindgen)
└── Cargo.toml           # Rust package manifest
```

## Next Steps

1. Read the [README](README.md) for detailed API documentation
2. Explore the examples in `examples/`
3. Run `cargo doc --open` to browse the full API documentation
4. Check out the C++ library documentation at the [parent repository](../)
