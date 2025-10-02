# Ruby Butler 🎩

*A distinguished Ruby environment manager with the refined approach of a gentleman's butler*

**⚠️ Prototype Status**: This is a prototype—not recommended for serious usage yet, even though the author has been using it daily for job work for some time. See [IDEAS.md](IDEAS.md) for potential future enhancements. Please share your ideas via issues.

**🎭 Development Fun**: The butler-themed language and refined terminology are just for entertainment during development and will be updated before any real production release, if any.

Your Ruby Butler doesn't reinvent the wheel, but rather composes the already well-known tools (`gem`, `bundle`) for your best experience—it serves you with distinction. Install your Rubies with your favourite installer like `ruby-install` or `ruby-build`, and those Rubies will be in good Butler's hands. Rather than modifying your shell environment, it meticulously prepares isolated environments, executes commands within them, and tidily cleans up afterward—just as a proper butler should serve.

## Hiring Your Butler (Installation)

Download the latest production binary for your platform from [releases](https://github.com/RubyElders/ruby-butler/releases) and add to PATH:

### Linux
```bash
curl -L https://github.com/RubyElders/ruby-butler/releases/latest/download/rb-linux -o ~/.local/bin/rb && chmod +x ~/.local/bin/rb
```

### macOS
```bash
curl -L https://github.com/RubyElders/ruby-butler/releases/latest/download/rb-macos -o ~/.local/bin/rb && chmod +x ~/.local/bin/rb
```

### Windows
```powershell
Invoke-WebRequest -Uri "https://github.com/RubyElders/ruby-butler/releases/latest/download/rb-windows.exe" -OutFile "$env:USERPROFILE\AppData\Local\Microsoft\WindowsApps\rb.exe"
```

## Quick Start

First, check your Ruby estate (by default checking at `~/.rubies`, see `--help` for more options):

```bash
# Survey your distinguished Ruby installations
rb runtime
```

Ruby Butler composes gem and bundle commands with environmental intelligence:

- **In Ruby environments**: Respects your Ruby environment selection
- **In Bundler environments**: Respects Gemfile and applies `bundle exec` automatically  
- **Ruby Detection**: Honors `.ruby-version` files and Gemfile ruby requirements

```bash
# Execute with latest Ruby (default behavior)
rb x ruby -v

# Execute with specific Ruby version
rb -r 3.4.5 x ruby -v

# Create a new Rails project with distinguished precision
rb x gem exec rails new butler-test
cd butler-test

# Synchronize bundler environment (handles bundle install)
rb sync

# Launch Rails console in the prepared environment  
rb x rails c

# Execute any Ruby command with proper environmental preparation
rb x gem list
rb x rake test
```

## Why Butler?

Unlike traditional Ruby managers that alter your shell environment, Ruby Butler employs a **service-oriented approach**:

- **No Environment Pollution**: Your shell remains pristine—Butler prepares environments per-execution
- **Automatic Bundler Integration**: Detects Gemfile projects and applies `bundle exec` intelligently  
- **Intelligent Composition**: Combines Ruby installations, gem environments, and bundler projects seamlessly
- **Graceful Error Handling**: Provides sophisticated guidance when commands or environments are missing

This approach mirrors how a distinguished butler operates: preparing everything behind the scenes, serving with precision, then disappearing without a trace.

## Installation Requirements

Ruby Butler expects Ruby installations in `~/.rubies/` (the standard location for `ruby-install` and similar tools). It discovers and composes:

- **Ruby Runtimes**: Installed via `ruby-install`, `ruby-build`, etc.
- **Gem Environments**: User gem directories (`~/.gem/ruby/X.Y.Z/`)  
- **Bundler Projects**: Detected automatically via `Gemfile` presence

## Commands

- `rb runtime` / `rb rt` - Survey your Ruby estate with elegant presentation
- `rb exec` / `rb x` - Execute commands within meticulously prepared environments
- `rb environment` / `rb env` - Display current environment composition
- `rb sync` - Manually synchronize bundler environments (auto-triggered when needed)
- `rb run` / `rb r` - Execute project scripts defined in `rbproject.toml`

## Configuration

- **`rb.toml`** - Global configuration file (in `%APPDATA%/rb/` or `~/.rb.toml`)
- **`rbproject.toml`** - Project-level script definitions and metadata

## Development

This is a **concept demonstration** showcasing a heavily opinionated approach to Ruby environment management.

### Building
```bash
cargo build --release
```

### Testing

**Rust Unit & Integration Tests:**
```bash
cargo test
```

**Shell Integration Tests (shellspec):**
```bash
./shellspec
```

**PowerShell Integration Tests (Pester):**
```bash
# Setup once
pwsh tests/Setup.ps1

# Run repeatedly  
Invoke-Pester tests/
```

### Architecture

- **`rb-core`**: Environment detection, composition, and orchestration
- **`rb-cli`**: Distinguished command-line interface with refined messaging
- **`rb-tests`**: Sophisticated testing utilities for realistic environment simulation

Ruby Butler is built with Rust for cross-platform reliability and employs a **environment-agnostic** design—no shell modifications required.

If you're curious about what Butler does under the hood, add `-v` or even better `-vv` to see the distinguished orchestration in action.

## Release Process

To create a new release with cross-platform binaries:

1. **Update version** in `crates/rb-cli/Cargo.toml`
2. **Update CHANGELOG.md** with release notes
3. **Create and push tag**:
   ```bash
   git tag v1.0.0
   git push --tags
   ```

The release workflow automatically:
- Builds binaries for Linux, macOS, and Windows (both release and debug)
- Creates GitHub release with binaries attached
- Embeds git information in version output (`rb --version`)

Released binaries include version traceability:
- **Tagged builds**: `Ruby Butler v1.0.0`
- **Development builds**: `Ruby Butler v0.1.0 (c5156b1) [debug build] [modified]`

---

*Distinguished Ruby development deserves distinguished tooling.*