# sparkplug-rs

Rust bindings for the Sparkplug B 2.2 protocol.

This library provides safe, ergonomic Rust wrappers around the C API of the [Sparkplug B C++ library](https://github.com/jsulmont/spark-plug_cpp). It enables Industrial IoT applications to publish and subscribe to Sparkplug messages over MQTT.

## About Sparkplug B

[Sparkplug B](https://sparkplug.eclipse.org/) is a specification for MQTT-enabled devices and applications to send and receive messages in a stateful way. It defines:

- Topic namespace structure for IIoT architectures
- Payload encoding using Protocol Buffers
- State management and sequence numbering
- Birth and Death certificates for device lifecycle tracking

**Resources:**
- [Sparkplug B Specification (PDF)](https://sparkplug.eclipse.org/specification/version/3.0/documents/sparkplug-specification-3.0.0.pdf)
- [C++ Implementation](https://github.com/jsulmont/spark-plug_cpp) - The underlying library this crate wraps
- [Eclipse Sparkplug Working Group](https://sparkplug.eclipse.org/)

## Features

- **Thread-safe**: All types implement `Send` + `Sync` (underlying C++ library is thread-safe)
- **RAII semantics**: Automatic resource cleanup via `Drop`
- **Type-safe**: Idiomatic Rust types and error handling with `Result<T, Error>`
- **Zero-copy where possible**: Efficient FFI bindings
- **Iterator support**: Iterate over metrics in payloads
- **Comprehensive API**: Full support for node and device lifecycle, commands, and state tracking

## Requirements

- Rust 2021 edition or later
- C++23 compiler (Clang 16+ or GCC 13+)
- CMake 3.25+
- Git (for fetching the C++ library)
- System dependencies: Eclipse Paho MQTT C library, Protocol Buffers, Abseil
- A running MQTT broker (e.g., Mosquitto)

## Installation

### From crates.io (once published)

```toml
[dependencies]
sparkplug-rs = "0.1"
```

### From source

```toml
[dependencies]
sparkplug-rs = { git = "https://github.com/jsulmont/sparkplug-rs" }
```

## Building

The build process is fully automated. Just run:

```bash
cargo build --release
```

This will automatically:

1. Clone the C++ library from https://github.com/jsulmont/spark-plug_cpp
2. Build the `sparkplug_c` shared library using CMake
3. Generate Rust FFI bindings
4. Build the Rust wrapper

No manual C++ library setup required!

### System Dependencies

**macOS (Homebrew):**

```bash
brew install cmake llvm protobuf abseil paho-mqtt-c
```

**Ubuntu/Debian:**

```bash
sudo apt-get install cmake clang-16 libc++-16-dev \
  libpaho-mqtt-dev protobuf-compiler libprotobuf-dev libabsl-dev
```

## Usage

### Publisher Example

```rust
use sparkplug_rs::{Publisher, PublisherConfig, PayloadBuilder, Result};

fn main() -> Result<()> {
    // Create publisher
    let config = PublisherConfig::new(
        "tcp://localhost:1883",
        "my_publisher",
        "Energy",
        "Gateway01"
    );

    let mut publisher = Publisher::new(config)?;
    publisher.connect()?;

    // Create NBIRTH with metrics and aliases
    let mut birth = PayloadBuilder::new()?;
    birth
        .add_double_with_alias("Temperature", 1, 20.5)
        .add_bool_with_alias("Active", 2, true);

    let birth_bytes = birth.serialize()?;
    publisher.publish_birth(&birth_bytes)?;

    // Publish NDATA updates using aliases
    let mut data = PayloadBuilder::new()?;
    data.add_double_by_alias(1, 21.0);

    let data_bytes = data.serialize()?;
    publisher.publish_data(&data_bytes)?;

    publisher.disconnect()?;
    Ok(())
}
```

### Subscriber Example

```rust
use sparkplug_rs::{Subscriber, SubscriberConfig, Message, Result};
use std::time::Duration;

fn main() -> Result<()> {
    let config = SubscriberConfig::new(
        "tcp://localhost:1883",
        "my_subscriber",
        "Energy"
    );

    let mut subscriber = Subscriber::new(config, Box::new(|msg: Message| {
        println!("Topic: {}", msg.topic);

        if let Ok(payload) = msg.parse_payload() {
            println!("Metrics: {}", payload.metric_count());

            for metric_result in payload.metrics() {
                if let Ok(metric) = metric_result {
                    println!("  {:?}", metric);
                }
            }
        }
    }))?;

    subscriber.connect()?;
    subscriber.subscribe_all()?;

    // Keep running to receive messages
    std::thread::sleep(Duration::from_secs(60));

    subscriber.disconnect()?;
    Ok(())
}
```

## Examples

Run the publisher example:

```bash
cargo run --example publisher
```

Run the subscriber example (in a separate terminal):

```bash
cargo run --example subscriber
```

Make sure you have a running MQTT broker (e.g., Mosquitto) on `localhost:1883`.

## Architecture

The library is organized into several modules:

- `Publisher`: Publish node and device data (NBIRTH, NDATA, DBIRTH, DDATA)
- `Subscriber`: Subscribe to messages with callback handlers
- `PayloadBuilder`: Build payloads with type-safe metric additions
- `Payload`: Parse and read received payloads with iterator support
- `types`: Common types (DataType, Metric, MetricValue)
- `error`: Error types and Result alias

## Thread Safety

All public types (`Publisher`, `Subscriber`, `PayloadBuilder`, `Payload`) are thread-safe and implement `Send` + `Sync`. This is guaranteed by the underlying C++ implementation which uses mutexes to protect all mutable state.

You can safely:

- Share publishers/subscribers across threads
- Publish from multiple threads simultaneously
- Call any method from any thread

## Documentation

Generate and view the documentation:

```bash
cargo doc --open
```

## License

MIT OR Apache-2.0

## Contributing

Contributions are welcome! Please see the main repository for contribution guidelines.
