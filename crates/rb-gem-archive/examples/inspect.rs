use std::{env, fs::File};

fn main() -> anyhow::Result<()> {
    let path = env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Usage: inspect <gem archive>"))?;
    println!("{:#?}", rb_gem_archive::read(File::open(path)?)?);
    Ok(())
}
