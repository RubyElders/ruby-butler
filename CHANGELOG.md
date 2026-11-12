# Changelog

All notable changes to Ruby Butler will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.2] - 2026-11-12

### Added
- Global `--format json` / `-F json` output for inspection, help, version, script listings, export and operation results. `RB_FORMAT` selects the default; explicit command-line options take precedence. JSON reports share a versioned envelope with command-specific data.
- CI artifacts for Linux, macOS and Windows release binaries, retained for seven days for development and integration testing.

### Changed
- Gemfile export uses `--project-format toml|kdl` to select the generated syntax independently of text/JSON presentation.
- Command data collection and text presentation use shared reports. Existing text layouts and stderr progress are preserved; executed programs and shell completion keep their own output.

## [0.4.1] - 2026-11-10

### Fixed
- Native preparation checks local gemspecs, loaded helpers and selected gem files instead of scanning entire repositories and worktrees. Gemfile snapshots also track loaded helpers and data files read during evaluation.
- Native installation accepts registry entries that omit Ruby requirements and checks the archive's actual requirement against the selected Ruby. Dependency mismatch errors identify the difference without dumping internal structures.
- Native lockfiles keep local path remotes relative and recognize equivalent gemspec paths across Windows separators and filesystem aliases.
- Native lockfile ordering, portable platform entries, Ruby/Bundler metadata and cached Bundler checksums follow Bundler.

## [0.4.0] - 2026-11-09

### Added
- `rb --help` displays help and suggests using `rb help`.
- **Ruby installation:** `rb install ruby 4.0` selects the newest matching stable prebuilt CRuby on Linux/macOS; `rb info runtime --available` lists downloadable versions. Downloads are cached, SHA-256 verified, safely extracted, and validated before installation. Existing complete installations are reused. The destination respects `-R`, `RB_RUBIES_DIR`, and `rubies-dir`, defaulting to `~/.rubies`.
- **Automatic Ruby preparation:** `rb sync`, `rb exec`/`rb x`, and `rb run`/`rb r` install missing Ruby runtimes before preparing dependencies. This is enabled by default on supported platforms, with configuration, environment, and command-line opt-outs.
- **Progress reporting:** live terminal trees show phases, parallel workers, recent build output, errors, human-readable transfer sizes, and consistent elapsed times. Nested operations share one display, including automatic Ruby installation followed by dependency preparation. Small terminals use a compact view, and redirected output uses an append-only log.
- **Gemfile export:** `rb export-gemfile` writes TOML to stdout; `--format kdl` writes KDL. Export supports sources, groups, platforms, Git/path dependencies, included Gemfiles, and local gemspecs, omitting inactive declarations. Gemfile discovery respects an explicit path, `BUNDLE_GEMFILE`, `gems.rb`/`Gemfile`, and global project/working-directory options.
- **Native package backend (opt-in):** resolves and installs dependencies from a Gemfile or project dependency declarations without Bundler performing resolution or installation. It supports registry, Git and path gems, platform-specific gems, and native extension compilation. Application runtime still uses RubyGems and Bundler.
- **Compatible lockfiles:** native preparation reuses valid locked graphs, resolves changed dependency graphs, and writes Bundler-compatible lockfiles. Existing target platforms are resolved together when refreshing a lock; only the local platform is installed.
- **Shared gem cache:** native downloads are cached per normalized registry source and SHA-256, verified before reuse, and shared across projects. Git sources use shared cached repositories and pinned snapshots; installed bundles remain project-local under `.rb`.
- **Private registries:** native preparation reads Bundler-compatible local, environment, and global credentials without loading Bundler. Command-based credential providers can be configured in `rb/auth.toml`, including commands that obtain a GitHub token. See [registry authentication](https://github.com/RubyElders/ruby-butler/blob/v0.4.0/crates/rb-packages/README.md).

### Changed
- Runtime discovery, project loading, and execution environment preparation are reused within each command, without sharing invocation state between runs.
- Explicit `rb sync` keeps its completed progress tree; automatic preparation before `exec` and `run` collapses to a summary. Already prepared native environments launch without repeating preparation.
- Native prepared state tracks dependency manifests, loaded dependency files, lockfiles, relevant environment reads, and declared path inputs. Application code and Ruby installation contents are not scanned. Unrelated launcher rebuilds do not invalidate prepared bundles.
- Environment information and executable completion use the selected package backend, including executables supplied by installed gems.
- Rust builds use a shared `rust-toolchain.toml`. Building from source now requires Rust 1.98.1; distributed binaries do not require Rust.

### Fixed
- `.ruby-version` accepts CRuby-prefixed versions such as `ruby-4.0.5` alongside `4.0.5`.
- Automatic Bundler preparation reconciles a stale lockfile even when all remaining Gemfile dependencies are already installed, including after removing a dependency.
- Package discovery, environment inspection, completion, and execution share backend selection and project paths.
- Native source selection, platform locks, archive identity checks, and private registry access received compatibility fixes.

### Enabling the new features

Bundler remains the default package backend. To make native preparation the default, add this to your global `rb.toml` (`$XDG_CONFIG_HOME/rb/rb.toml`, normally `~/.config/rb/rb.toml`; `%APPDATA%/rb/rb.toml` on Windows):

```toml
package-backend = "native"
```

For KDL config (`rb.kdl`), use `package-backend "native"`. For one command, use `rb --package-backend native sync`; for a shell session, export `RB_PACKAGE_BACKEND=native`. This also selects the backend for `rb x` and `rb run`. Set the value to `bundler` to switch back. Command-line settings override environment variables, which override file settings. Run `rb info config` to inspect effective settings and their sources.

Automatic Ruby installation is already enabled. To require an existing Ruby, add `auto-install-ruby = false` to your global TOML config, export `RB_AUTO_INSTALL_RUBY=false`, or pass `--no-auto-install-ruby`. Use `--auto-install-ruby` for a one-command override. KDL uses `auto-install-ruby false`.

Ruby installation on Windows and installation from source are not supported. Native gem compilation needs the selected Ruby's development headers, a compiler, and any system libraries required by the gem. The native backend does not yet support Bundler plugins, custom lockfile paths, differing registry sources for the same gem across targets, or MicroGems as the application runtime.

### Libraries

The new standalone crates separate work from presentation and can be used independently of the CLI:

| Crate | Purpose |
| --- | --- |
| `rb-task` | Dependency graphs, bounded parallel execution, lifecycle/progress/output events, and typed failures. |
| `rb-progress` | Terminal and plain reporting, nested operations, worker output, and optional `rb-task` integration. |
| `rb-gemfile` | Trusted Gemfile evaluation, typed manifests, cached evaluation, and TOML/KDL export. |
| `rb-compact-index` | Registry metadata and archive access, source-scoped caches, ETags, checksum verification, and legacy index fallback. |
| `rb-gem-types` | Ruby-compatible gem versions, requirements, platforms, and package identities. |
| `rb-resolver` | PubGrub resolution with Ruby/platform filtering, locked preferences, prereleases, and multiple targets. |
| `rb-lockfile` | Parsing and deterministic rendering of registry, Git, path, platform, and checksum lockfile sections. |
| `rb-gem-archive` | Bounded `.gem` metadata reading and archive member checksum verification without Ruby. |
| `rb-gem-installer` | Staged payload installation and extconf/make, rake/mkrf, configure, CMake, and Cargo builds. |
| `rb-microgems` | Minimal RubyGems-compatible APIs for isolated native extension build subprocesses. |
| `rb-packages` | Manifest loading, source acquisition, resolution, lockfiles, installation, and prepared-state reuse. |
| `rb-ruby-installer` | Cached, verified prebuilt CRuby installation from rv-ruby distributions. |

The Ruby helpers live under `gems/`, with independent Ruby tests and linting, and are embedded in the distributed executable. This release does not publish them as Ruby gems or publish the Rust crates to crates.io.

## [0.3.1] - 2026-05-03

### Fixed
- `rb sync` now relies on `BUNDLE_PATH` instead of rewriting Bundler's local path configuration, preventing the repeated "replacing the current local value of path" warning on subsequent bundle checks

## [0.3.0] - 2026-03-03

### Added
- Bash shell completion with context-aware suggestions via `rb shell-integration bash`
- Working directory flag `-C, --work-dir` to run commands from a different directory
- `rb info` diagnostic subcommands: `runtime`, `env`, `project`, `config`
- `rb version` command with git hash, build profile, and dirty state display
- Custom help system with grouped command display and gentleman's formatting
- Sophisticated error display with platform-specific guidance for bundler failures
- Regression test for bundler deprecation warnings
- macOS ARM64 (aarch64) release builds
- Shell script linting (ShellCheck) and YAML linting in CI
- PowerShell script linting (PSScriptAnalyzer) in CI

### Changed
- Rewritten CLI structure with command dispatch, separated help, and refined output
- Ruby detection rewritten with strategy pattern (version file, Gemfile, CLI flag)
- Gem path detection split into focused strategies (bundler isolation, custom base, user gems)
- Bundler sync now cleans removed gems from lockfile automatically
- Updated `bundle config path` to `bundle config set path` for modern Bundler compatibility
- Test environment upgraded to Ruby 4.0.1 + 3.4.5 (from 3.2.4 + 3.4.5)
- CI now builds Docker image locally on PRs when Dockerfile changes

### Fixed
- Bundler deprecation warning with Ruby 4.0+ (`bundle config` → `bundle config set`)

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
