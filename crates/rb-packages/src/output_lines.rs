#[derive(Default)]
pub(crate) struct OutputLines(Vec<u8>);

impl OutputLines {
    pub(crate) fn push(&mut self, bytes: &[u8], mut output: impl FnMut(String)) {
        self.0.extend_from_slice(bytes);
        while let Some(position) = self.0.iter().position(|byte| *byte == b'\n') {
            let line: Vec<_> = self.0.drain(..=position).collect();
            output(
                String::from_utf8_lossy(&line[..line.len() - 1])
                    .trim_end_matches('\r')
                    .to_owned(),
            );
        }
        if self.0.len() >= crate::constants::MAX_OUTPUT_LINE_BYTES {
            self.finish(output);
        }
    }

    pub(crate) fn finish(&mut self, mut output: impl FnMut(String)) {
        if !self.0.is_empty() {
            output(String::from_utf8_lossy(&self.0).into_owned());
            self.0.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn read_boundaries_do_not_split_utf8_or_create_extra_lines() {
        let mut lines = OutputLines::default();
        let mut output = Vec::new();
        lines.push(&[b'a', 0xc3], |line| output.push(line));
        lines.push(&[0xa9, b'\r', b'\n', b'b'], |line| output.push(line));
        lines.finish(|line| output.push(line));
        assert_eq!(output, ["a\u{e9}", "b"]);
    }
}
