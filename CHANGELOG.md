# Changelog

All notable changes to Ruby Butler will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-10-19

### Added
- Configuration file support via `rb.toml` or `rb.kdl` for global settings
- Project script management with `rbproject.toml` or `rbproject.kdl` and `rb run` command
- Project bootstrap script with `rb init`
- Alternative project file naming: `gem.toml` or `gem.kdl` (for gem development)
- Short flag `-B` for `--no-bundler` option

## [0.1.0] - 2025-09-26

### Added
- Core Ruby environment detection and orchestration
- Ruby runtime discovery with semver-based version selection
- Gem environment management with proper PATH composition
- Bundler project detection and automatic `bundle exec` integration
- Distinguished command-line interface with gentleman's butler persona
- Cross-platform support (Windows, macOS, Linux)
- Environment composition without shell modification
- Comprehensive error handling with sophisticated messaging
- Git version embedding in binaries for traceability
- Automated release workflow with cross-platform binaries
- Complete test suite with unit, integration, and shell tests

---

*Distinguished releases crafted with appropriate ceremony by RubyElders.com*
