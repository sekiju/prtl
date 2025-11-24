# Contributing

Thank you for your interest in contributing!

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR-sekiju/prtl.git
   cd prtl
   ```
3. **Create a branch** for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Setup

### Prerequisites

- Rust 1.83+ (edition 2024)
- Docker and Docker Compose (for testing)
- NATS and DragonflyDB (can be started via docker-compose)

### Running Locally

```bash
# Start dependencies
docker-compose up -d

# Copy environment configuration
cp .env.example .env

# Build and run
cargo build
cargo run --bin api
```

## Code Quality

Before submitting a PR, please ensure:

1. **Code formatting**: Run `cargo fmt`
   ```bash
   cargo fmt --all
   ```

2. **Linting**: Run `cargo clippy` with no warnings
   ```bash
   cargo clippy --all-targets --all-features --workspace -- -D warnings
   ```

3. **Tests**: All tests pass
   ```bash
   cargo test --all-features --workspace
   ```

4. **Build**: Project builds successfully
   ```bash
   cargo build --release --all-features --workspace
   ```

## Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `refactor:` Code refactoring
- `test:` Adding or updating tests
- `chore:` Maintenance tasks

Example:
```
feat(proxy): add support for custom headers
fix(api): resolve race condition in proxy registration
docs: update installation instructions
```

## Pull Request Process

1. **Update documentation** if you've changed APIs or added features
2. **Add tests** for new functionality
3. **Update CHANGELOG.md** with a note describing your changes
4. **Link related issues** in the PR description
5. **Request review** from maintainers

### PR Checklist

- [ ] Code follows the project's style guidelines
- [ ] Self-review of the code completed
- [ ] Comments added for complex logic
- [ ] Documentation updated
- [ ] Tests added/updated and passing
- [ ] No new warnings from clippy
- [ ] CHANGELOG.md updated

## Project Structure

```
mirror-eroge-dev/
├── bin/api/              # Main API service
├── lib/
│   ├── mirror-messages/  # Shared message protocol
│   └── mirror-proxy/     # Core proxy functionality
└── examples/
    └── proxy-cdnlibs/    # Reference proxy implementation
```

## Architecture Guidelines

- **Services communicate via NATS** - Use `mirror-messages` for all inter-service communication
- **Proxies are dynamic** - Register/unregister at runtime
- **Use async/await** - All I/O should be asynchronous with Tokio
- **Error handling** - Use `Result` types and proper error propagation

## Testing

### Unit Tests
```bash
cargo test --lib
```

### Integration Tests
```bash
# Ensure dependencies are running
docker-compose up -d
cargo test --test '*'
```

### End-to-End Tests
```bash
# Start all services
docker-compose up -d
cargo run --bin api &
cargo run --bin proxy-cdnlibs &
# Run your tests...
```

## Getting Help

- **Questions?** Open a [Discussion](https://github.com/sekiju/prtl/discussions)
- **Found a bug?** Open an [Issue](https://github.com/sekiju/prtl/issues)
- **Need clarification?** Comment on relevant PRs or Issues

## Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). Please be respectful and constructive.

## License

By contributing, you agree that your contributions will be dual-licensed under MIT and Apache-2.0, matching the project's license.

---

Thank you for contributing!
