# Ruby Butler Ideas & Future Features

This document contains a curated collection of ideas and potential enhancements for Ruby Butler's evolution.

## Central Configuration System
- [ ] **Global `rb.toml` configuration file**
  - Respect XDG Base Directory Specification on Unix systems  
  - Override default settings like Ruby installation directories
  - Configure default Ruby version preferences

- [ ] **Local `rb.toml` project files**
  - Project-specific Ruby version requirements
  - Basic project information (name, version, description, author, license)
  - Custom project scripts (npm-inspired)

## npm-Inspired Script System
- [ ] **Custom project scripts in `rb.toml`**
  - `rb setup` - Project initialization and dependency installation
  - `rb lint` - Code quality checks (RuboCop, etc.)
  - `rb test` - Test suite execution
  - `rb build` - Build/compile project artifacts
  - Custom user-defined scripts with project-specific meanings

## TOML-Based Gemfile Alternative
- [ ] **Merge Gemfile functionality into `rb.toml`**
  - Dependencies section with version constraints
  - Development vs production dependency groups
  - Platform-specific dependencies
  - Maintain compatibility with existing Gemfile ecosystem

## Enhanced Dependency Management
- [ ] **Custom install/resolve implementation for extra speed**
  - Faster dependency resolution algorithms
  - Parallel gem downloads and installations
  - Better conflict resolution strategies

## Ruby Installation Management
- [ ] **Proxy ruby installing to other tools and auto-install when missing**
  - Detect and proxy to `ruby-install`, `ruby-build`, `rbenv`, etc.
  - Automatic Ruby version installation when missing
  - Support for multiple Ruby installation backends

## Project Generator
- [ ] **Simple project scaffolding**
  - `rb new <project>` - Create new Ruby projects with templates
  - Simple gem skeleton generation (proxy to `bundle gem` when appropriate)
  - Custom project templates with `rb.toml` configuration
  - Template system for different project types (gem, app, script)