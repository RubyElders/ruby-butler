use rb_lockfile::Lockfile;
use std::{env, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .ok_or("Usage: roundtrip <Gemfile.lock>")?;
    let lock = Lockfile::parse(&fs::read_to_string(path)?)?;
    print!("{}", lock.render()?);
    Ok(())
}
