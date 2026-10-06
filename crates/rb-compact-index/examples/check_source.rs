use anyhow::{Context, Result, ensure};
use rb_compact_index::Client;

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let source = args
        .get(1)
        .context("Usage: check_source SOURCE GEM [VERSION]")?;
    let name = args.get(2).context("Missing gem name")?;
    let cache = tempfile::tempdir()?;
    let client = Client::new(source, cache.path(), false)?;
    let records = client.info(name)?;
    ensure!(
        client.info(name)? == records,
        "Metadata changed during revalidation"
    );
    let selected = if let Some(version) = args.get(3) {
        records.iter().find(|record| &record.version == version)
    } else {
        records.last()
    }
    .context("Requested version is absent or source has no versions")?;
    let checksum = selected
        .metadata
        .get("checksum")
        .context("Missing archive checksum")?;
    let archive = client.download(name, &selected.version, checksum)?;
    let offline = Client::new(source, cache.path(), true)?;
    ensure!(offline.info(name)? == records, "Offline metadata differs");
    ensure!(
        offline.download(name, &selected.version, checksum)? == archive,
        "Offline archive differs"
    );
    println!(
        "{}",
        serde_json::json!({
            "source": client.source(), "gem": name, "versions": records.len(),
            "sample_version": selected.version, "archive_bytes": std::fs::metadata(archive)?.len(),
            "metadata_fields": selected.metadata.keys().collect::<Vec<_>>(),
            "revalidation": true, "offline": true
        })
    );
    Ok(())
}
