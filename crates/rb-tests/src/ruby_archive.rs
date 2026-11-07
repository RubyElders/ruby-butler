use flate2::{Compression, write::GzEncoder};
use std::io::{self, Cursor, Write};

pub struct RubyArchive {
    root: String,
    archive: tar::Builder<Vec<u8>>,
}

impl RubyArchive {
    pub fn new(version: &str) -> Self {
        Self {
            root: format!("rv-ruby@{version}/{version}"),
            archive: tar::Builder::new(Vec::new()),
        }
    }

    pub fn file(
        &mut self,
        name: &str,
        contents: impl AsRef<[u8]>,
        mode: u32,
    ) -> io::Result<&mut Self> {
        let contents = contents.as_ref();
        let mut header = tar::Header::new_gnu();
        header.set_size(contents.len() as u64);
        header.set_mode(mode);
        header.set_cksum();
        self.archive.append_data(
            &mut header,
            format!("{}/{name}", self.root),
            Cursor::new(contents),
        )?;
        Ok(self)
    }

    pub fn symlink(&mut self, name: &str, target: &str) -> io::Result<&mut Self> {
        self.link(name, target, tar::EntryType::Symlink)
    }

    pub fn hardlink(&mut self, name: &str, target: &str) -> io::Result<&mut Self> {
        self.link(name, target, tar::EntryType::Link)
    }

    fn link(&mut self, name: &str, target: &str, kind: tar::EntryType) -> io::Result<&mut Self> {
        let mut header = tar::Header::new_gnu();
        header.set_size(0);
        header.set_mode(0o777);
        header.set_entry_type(kind);
        self.archive
            .append_link(&mut header, format!("{}/{name}", self.root), target)?;
        Ok(self)
    }

    pub fn finish(self) -> io::Result<Vec<u8>> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&self.archive.into_inner()?)?;
        encoder.finish()
    }
}
