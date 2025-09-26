# Copilot Instructions for ruby-butler2

## Project Overview
- **Language:** Rust (Cargo workspace with 2024 edition)
- **Purpose:** A sophisticated Ruby environment manager (not version switcher) that orchestrates Ruby installations and gem collections with distinguished precision
- **Inspiration:** Cargo, uv, and npm - environment-agnostic tools that transcend shell-based approaches
- **Tone:** Gentleman's butler - refined, sophisticated, helpful, and elegant in all communications

## Main Components
- **`crates/rb-core/`**: Core library for Ruby/Gem environment detection and orchestration
  - `src/butler/`: Environment composition and orchestration (ButlerRuntime)
  - `src/ruby/`: Ruby runtime detection and abstraction (RubyRuntimeDetector)
  - `src/gems/`: RubyGems runtime management (GemRuntime)
  - `tests/`: Integration and unit tests with `RubySandbox` helpers
- **`crates/rb-cli/`**: Distinguished command-line interface
  - `src/bin/rb.rs`: Main binary entry point
  - `src/commands/`: Modular command implementations (runtime, exec)
  - `src/lib.rs`: CLI types, argument parsing with refined help text
  - `tests/integration_tests.rs`: End-to-end CLI testing
- **`crates/rb-tests/`**: Sophisticated testing utilities
  - `src/sandbox.rs`: `RubySandbox` for filesystem-based test setups

## Architecture & Design Philosophy
- **Environment Composition**: Each runtime implements `EnvProvider` trait to expose environment modifications (PATH, GEM_HOME, GEM_PATH)
- **Ruby Discovery**: `RubyRuntimeDetector::discover()` scans directories for Ruby installations using semver sorting
- **Butler Pattern**: `ButlerRuntime` aggregates all providers and composes the final environment for process execution
- **Environment Agnostic**: No shell dependencies - works universally across Windows, macOS, and Linux
- **Gentleman's Approach**: All user-facing text uses refined, sophisticated language befitting a distinguished developer

## CLI Commands & User Interface
- **`rb runtime` / `rb rt`**: Survey and present Ruby estate with elegant formatting
- **`rb exec` / `rb x`**: Execute commands within meticulously prepared Ruby environments
- **Help System**: Cargo/uv-inspired styling with gentleman's language throughout
- **Error Messages**: Refined, helpful guidance rather than technical jargon
- **Output Formatting**: Clean alignment, appropriate use of color, dignified presentation

## Language & Tone Guidelines
**Always maintain the gentleman's butler persona:**
- Use refined vocabulary: "distinguished", "meticulously prepared", "with appropriate dignity"
- Avoid technical jargon in user-facing messages
- Present information with ceremony and proper formatting
- Use "Ruby environment manager" never "version manager"
- Error messages should be helpful and courteous: "Selection Failed" not "Error"
- Success messages should be celebratory: "Environment Ready" with context
- Refer to gem directories as "Gem home" (correct technical term)
- Maintain consistency: "Installation", "Gem home", "Gem libraries", "Executable paths"

## Developer Workflows
- **Build**: `cargo build` (workspace root) or `cargo build --package <crate>`
- **Test**: `cargo test` for all, `cargo test --package <crate>` for specific
- **Debug**: Use `--show-output` for test debugging, `cargo run --bin rb -- <args>` for CLI testing
- **Integration Tests**: Use `RubySandbox` for realistic filesystem-based testing

## Project-Specific Patterns
- **Environment Variables**: 
  - `GEM_HOME` and `GEM_PATH` always set to same value (per chruby pattern)
  - PATH composition includes Ruby bin and gem bin directories
  - All environment composition through `ButlerRuntime::env_vars()`
- **Testing Conventions**:
  - Integration tests use `RubySandbox` for filesystem simulation
  - Place test helpers in dedicated test crates (`rb-tests`)
  - Test both happy path and error conditions with proper assertions
- **Code Style**:
  - Imports at top of files
  - Tests at end of files  
  - Use descriptive variable names reflecting the gentleman's approach
  - Error handling with helpful context for users

## Key Technical Details
- **Ruby Detection**: Scans for `ruby-X.Y.Z` directories, validates with `bin/ruby` executable
- **Gem Runtime**: Infers from Ruby version using standard gem directory patterns
- **Environment Building**: Composes PATH, GEM_HOME, GEM_PATH for subprocess execution
- **Version Handling**: Uses `semver` crate for proper version parsing and comparison
- **Cross-Platform**: Handles Windows/Unix path separators and executable extensions

## Integration Points
- **No External Dependencies**: All logic is local and filesystem-based
- **Ruby Ecosystem**: Compatible with standard Ruby installation patterns (ruby-install, etc.)
- **Gem Management**: Works with bundler, gem commands, and standard RubyGems workflows
- **Development Tools**: Integrates cleanly with cargo-style development workflows

## Key Files & Responsibilities
- `crates/rb-core/src/butler/mod.rs`: Main environment composition logic
- `crates/rb-core/src/butler/env_provider.rs`: Environment modification trait
- `crates/rb-core/src/ruby/detector.rs`: Ruby installation discovery
- `crates/rb-core/src/gems/mod.rs`: Gem environment management
- `crates/rb-cli/src/lib.rs`: CLI argument parsing and help text
- `crates/rb-cli/src/commands/runtime.rs`: Ruby estate surveying and presentation
- `crates/rb-cli/src/commands/exec.rs`: Command execution in composed environments
- `crates/rb-tests/src/sandbox.rs`: Test environment simulation

## Quality Standards
- **All tests must pass**: Unit tests, integration tests, and end-to-end CLI tests
- **Consistent messaging**: All user-facing text follows gentleman's butler tone
- **Clean formatting**: Perfect text alignment, appropriate use of color and styling
- **Cross-platform compatibility**: Windows, macOS, and Linux support
- **Error resilience**: Graceful handling of missing directories, invalid versions, etc.
