fn main() -> anyhow::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let archive = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("Expected archive path"))?;
    let destination = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("Expected destination path"))?;
    let metadata = rb_gem_installer::install(std::fs::File::open(archive)?, destination)?;
    println!("Installed {}", metadata.id);
    Ok(())
}
