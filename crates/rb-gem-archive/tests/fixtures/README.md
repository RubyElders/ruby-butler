# Gem metadata fixtures

Captured from three public RubyGems.org archives on 2026-10-08. `sources.json` records their URLs and archive and metadata SHA-256 values. Only original decompressed gemspec YAML is retained; binary payloads remain outside the test fixtures. The metadata remains subject to each gem's original license.

Expected fields were read from the full archives using RubyGems 3.6.9 `Gem::Package#spec` on Ruby 3.4.10, using `raw_require_paths` for declared paths rather than installation-specific extension directories. The captures cover pure Ruby rake, bigdecimal with native extension declarations, and Java-platform json. Tests rebuild small containers around those metadata bytes and compare every exposed field. Separately, the complete downloaded archives were read locally to verify their original member checksums.

`metadata.yml` is synthetic tagged Ruby YAML for duplicate requirements and development-dependency filtering. The reader treats tags as data and does not instantiate Ruby objects.
