# Contributing to HashLoader

Thanks for your interest in improving HashLoader! Contributions are welcome, but changes are incorporated into the original repository only after they are reviewed and accepted by the maintainer.

## Before you start

- For a substantial change or a new feature, open an [issue](https://github.com/RedexDev/HashLoader/issues) first to discuss the approach.
- Before reporting a bug, search existing issues to see whether it has already been reported.
- Keep each pull request focused on one change where possible.

## Submitting a change

1. Fork the repository and create a branch for your change.
2. Make your changes and add or update tests or documentation when appropriate.
3. Run the relevant checks listed below.
4. Open a pull request against `RedexDev/HashLoader` and explain what changed, why, and how you tested it. Link any related issue.

Please do not include unrelated changes. The maintainer will review pull requests and decide whether to merge them.

## Development setup

HashLoader is primarily a Rust project. Install the stable Rust toolchain, clone your fork, and run the test suite:

```sh
git clone https://github.com/<your-github-username>/HashLoader.git
cd HashLoader
cargo test
```

For platform-specific build requirements and instructions for rebuilding the Haxe payload or native bridge, see [Building and Development](docs/BUILDING.md).

## Useful checks

Run these from the repository root before submitting a pull request:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features
cargo test --all-features
```

If your changes affect a particular platform or build artifact, test that path too when possible and mention any limitations in your pull request.

## Documentation

Update the relevant documentation when changing user-facing behavior, installation steps, the modding workflow, or build requirements. Include examples or screenshots when they make the change easier to understand.
