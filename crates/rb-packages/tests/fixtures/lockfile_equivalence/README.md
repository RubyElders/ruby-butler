# Lockfile comparison

The integration test resolves this Gemfile independently with native preparation and real Bundler. Both use a temporary localhost registry containing real gem archives, a local Git repository and relative path gems. No lockfile is supplied to either initial resolution. The complete lockfile strings are compared without normalization, then native preparation reuses Bundler's lock offline in locked mode.

Bundler receives `--add-checksums` so checksum generation is enabled even on older 2.6 releases. Platform entries and the optional checksum for Bundler itself are included unchanged in the comparison. Bundler 2.6 or newer is required.
