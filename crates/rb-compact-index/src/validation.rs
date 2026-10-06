use anyhow::{Result, ensure};

pub(crate) fn validate_name(name: &str) -> Result<()> {
    ensure!(
        name.bytes().any(|c| c.is_ascii_alphanumeric())
            && name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c)),
        "Invalid or unsupported gem name: {name:?}"
    );
    Ok(())
}

pub(crate) fn validate_checksum(value: &str) -> Result<()> {
    ensure!(
        value.len() == 64
            && value
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "Invalid SHA256 checksum"
    );
    Ok(())
}
