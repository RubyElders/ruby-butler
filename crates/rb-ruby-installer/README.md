# rb-ruby-installer

Installs prebuilt CRuby distributions from [rv-ruby](https://github.com/spinel-coop/rv-ruby) on Linux and macOS without requiring Ruby or rv.

```rust,no_run
use rb_ruby_installer::{Installer, Platform};

let installer = Installer::new("/home/me/.cache/rb/rubies")?;
let ruby = installer.install(
    &"3.4".parse()?,
    Platform::current()?,
    std::path::Path::new("/home/me/.rubies"),
    |event| eprintln!("{event:?}"),
)?;
println!("{}", ruby.path.display());
# Ok::<(), anyhow::Error>(())
```

Downloads are cached and checked against published SHA-256 values. Extraction and runtime validation happen in staging before the installation becomes visible. Existing complete installations are reused. Progress is supplied as events for the caller to present.

Windows installation and source builds are not supported. Native gem compilation still needs a system compiler and any libraries required by the gem.
