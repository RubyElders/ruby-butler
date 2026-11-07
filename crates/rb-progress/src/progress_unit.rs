#[derive(Clone, Copy, Default)]
pub(crate) enum ProgressUnit {
    #[default]
    Count,
    Bytes,
    Status,
}

impl ProgressUnit {
    pub(crate) fn format(self, done: usize, total: usize) -> String {
        match self {
            Self::Status => String::new(),
            Self::Count => format!("{done}/{total}"),
            Self::Bytes if total == 0 => format_bytes(done),
            Self::Bytes => format!(
                "{} / {} ({:.0}%)",
                format_bytes(done),
                format_bytes(total),
                (done as f64 / total as f64 * 100.0).min(100.0)
            ),
        }
    }
}

fn format_bytes(bytes: usize) -> String {
    const UNITS: [&str; 7] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
    let mut value = bytes as f64;
    let mut index = 0;
    while value >= 1024.0 && index + 1 < UNITS.len() {
        value /= 1024.0;
        index += 1;
    }
    if index == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formats_counts_bytes_and_unknown_totals() {
        assert_eq!(ProgressUnit::Count.format(2, 3), "2/3");
        for (done, total, text) in [
            (0, 0, "0 B"),
            (128, 256, "128 B / 256 B (50%)"),
            (1024, 2048, "1.0 KiB / 2.0 KiB (50%)"),
            (
                13 * 1024 * 1024,
                50 * 1024 * 1024,
                "13.0 MiB / 50.0 MiB (26%)",
            ),
            (1024 * 1024 * 1024, 0, "1.0 GiB"),
        ] {
            assert_eq!(ProgressUnit::Bytes.format(done, total), text);
        }
    }
}
