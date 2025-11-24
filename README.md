# sekiju/prtl

[![Docker](https://github.com/sekiju/prtl/actions/workflows/docker-publish.yml/badge.svg)](https://github.com/sekiju/prtl/actions/workflows/docker-publish.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

A modern proxy-based architecture for content mirroring and caching, built with Rust and NATS messaging.

## Features

- **Proxy-based architecture** - Modular proxy services with dynamic registration
- **Message-driven** - NATS-based communication between services
- **Fast caching** - DragonflyDB-backed caching layer

## Installation

### Using Docker

```bash
# Pull the latest image
docker pull ghcr.io/sekiju/prtl:latest

# Run as a container
docker run -d --name prtl -p 80:80 ghcr.io/sekiju/prtl:latest
```

### Building from Source

```bash
# Prerequisites: Rust 1.83+ (edition 2024)
git clone https://github.com/sekiju/prtl.git
cd mirror

# Build
cargo build --release

# Run
./target/release/api
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is dual-licensed under:
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

You may choose either license at your option.
