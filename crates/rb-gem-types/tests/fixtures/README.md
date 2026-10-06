# RubyGems oracle

`cases.json` defines the input corpus. `rubygems.json` is the unchanged output of
`oracle.rb`, captured with MRI 4.0.6 and RubyGems 4.0.20 on 2026-10-07.
It covers version ordering, release/bump, requirements and directional platform
matching. Pure Ruby platform matching uses the package-selection convention;
RubyGems itself represents that platform as a string rather than an object.

To regenerate after inspecting a RubyGems behavior change:

```sh
ruby crates/rb-gem-types/tests/fixtures/oracle.rb \
  crates/rb-gem-types/tests/fixtures/cases.json \
  > crates/rb-gem-types/tests/fixtures/rubygems.json
```

The live integration test compares version and requirement cases against
installed RubyGems. Platform cases use the pinned snapshot because older
RubyGems releases parse some platform strings differently.
Neither test downloads or installs gems.
