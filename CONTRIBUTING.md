# Contributing to CCPlayer

First off, thank you for considering contributing to CCPlayer! It's people like you that make CCPlayer such a great tool.

## Code of Conduct

This project and everyone participating in it is governed by the CCPlayer Code of Conduct. By participating, you are expected to uphold this code. Please report unacceptable behavior to [conduct@datalos.com](mailto:conduct@datalos.com).

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check the existing issues as you might find out that you don't need to create one. When you are creating a bug report, please include as many details as possible:

* **Use a clear and descriptive title** for the issue to identify the problem.
* **Describe the exact steps which reproduce the problem** in as many details as possible.
* **Provide specific examples to demonstrate the steps**.
* **Describe the behavior you observed after following the steps** and point out what exactly is the problem with that behavior.
* **Explain which behavior you expected to see instead and why.**
* **Include screenshots and animated GIFs** which show you following the described steps and clearly demonstrate the problem.
* **If the problem is related to performance or memory**, include a CPU profile capture with your report.
* **Include crash reports** with a stack trace from the operating system.

### Suggesting Enhancements

Before creating enhancement suggestions, please check the existing issues and discussions. When you are creating an enhancement suggestion, please include as many details as possible:

* **Use a clear and descriptive title** for the issue to identify the suggestion.
* **Provide a step-by-step description of the suggested enhancement** in as many details as possible.
* **Provide specific examples to demonstrate the steps**.
* **Describe the current behavior** and **explain which behavior you expected to see instead** and why.
* **Include screenshots and animated GIFs** which help you demonstrate the steps or point out the part of CCPlayer which the suggestion is related to.
* **Explain why this enhancement would be useful** to most CCPlayer users.

### Pull Requests

* Fill in the required template
* Do not include issue numbers in the PR title
* Include screenshots and animated GIFs in your pull request whenever possible
* Follow the Rust style guide
* Include tests when adding new features
* Update documentation when needed
* End all files with a newline

## Development Setup

1. Fork the repo and create your branch from `main`.
2. Install Rust (1.75+) and FFmpeg dependencies.
3. Run `cargo build` to ensure everything compiles.
4. Make your changes.
5. Run tests with `cargo test`.
6. Run `cargo fmt` and `cargo clippy` before committing.
7. Push to your fork and submit a pull request.

## Style Guide

### Rust Style Guide

* Use `cargo fmt` to format your code
* Use `cargo clippy` to catch common mistakes
* Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
* Write doc comments for all public items
* Use meaningful variable and function names
* Keep functions small and focused
* Prefer composition over inheritance

### Git Commit Messages

* Use the present tense ("Add feature" not "Added feature")
* Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
* Limit the first line to 72 characters or less
* Reference issues and pull requests liberally after the first line
* Consider starting the commit message with an applicable emoji:
  * 🎨 `:art:` when improving the format/structure of the code
  * 🐎 `:racehorse:` when improving performance
  * 📝 `:memo:` when writing docs
  * 🐛 `:bug:` when fixing a bug
  * 🔥 `:fire:` when removing code or files
  * ✅ `:white_check_mark:` when adding tests
  * 🔒 `:lock:` when dealing with security
  * ⬆️ `:arrow_up:` when upgrading dependencies
  * ⬇️ `:arrow_down:` when downgrading dependencies

## Testing

* Write unit tests for all new functionality
* Ensure all tests pass before submitting PR
* Add integration tests for complex features
* Test on multiple platforms if possible

## Documentation

* Update README.md if needed
* Add inline documentation for complex code
* Update CHANGELOG.md for notable changes
* Keep examples up to date

Thank you for contributing to CCPlayer! 🎬