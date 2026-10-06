use rb_gemfile::Evaluator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gemfile = std::env::args().nth(1).unwrap_or_else(|| "Gemfile".into());
    let ruby = std::env::var_os("RB_GEMFILE_RUBY").unwrap_or_else(|| "ruby".into());
    let result = Evaluator::new(ruby).evaluate(gemfile)?;
    eprint!("{}{}", result.stdout, result.stderr);
    print!("{}", result.manifest.to_toml()?);
    Ok(())
}
