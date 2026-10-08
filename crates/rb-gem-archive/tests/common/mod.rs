use flate2::{Compression, write::GzEncoder};
use sha2::{Digest, Sha256};
use std::io::Write;

pub fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

pub fn container(members: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut tar = tar::Builder::new(Vec::new());
    for (name, bytes) in members {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, name, bytes.as_slice())
            .unwrap();
    }
    tar.into_inner().unwrap()
}

pub fn members(metadata: &[u8]) -> Vec<(&'static str, Vec<u8>)> {
    let metadata = gzip(metadata);
    let data = gzip(&container(&[]));
    let checksums = format!(
        "SHA256:\n  metadata.gz: {:x}\n  data.tar.gz: {:x}\n",
        Sha256::digest(&metadata),
        Sha256::digest(&data)
    );
    vec![
        ("metadata.gz", metadata),
        ("data.tar.gz", data),
        ("checksums.yaml.gz", gzip(checksums.as_bytes())),
    ]
}
