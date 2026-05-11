use anyhow::{Context, Result};
use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use zip::ZipArchive;

pub struct ZipPageSource {
    archive: ZipArchive<BufReader<File>>,
    entries: Vec<String>,
}

impl ZipPageSource {
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
        let archive = ZipArchive::new(BufReader::new(file))?;

        let mut entries: Vec<String> = archive
            .file_names()
            .filter(|n| is_image(n))
            .map(|s| s.to_string())
            .collect();
        entries.sort_by(|a, b| natord_cmp(a, b));

        Ok(Self { archive, entries })
    }

    pub fn page_count(&self) -> usize {
        self.entries.len()
    }

    pub fn entry_name(&self, idx: usize) -> &str {
        &self.entries[idx]
    }

    pub fn page_bytes(&mut self, idx: usize) -> Result<Vec<u8>> {
        let name = self.entries[idx].clone();
        let mut entry = self.archive.by_name(&name)?;
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf)?;
        Ok(buf)
    }
}

fn is_image(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    matches!(
        Path::new(&lower).extension().and_then(|s| s.to_str()),
        Some("jpg" | "jpeg" | "png" | "webp" | "bmp" | "gif")
    )
}

/// Natural ordering: compare digit runs as numbers so "page_2" < "page_10".
fn natord_cmp(a: &str, b: &str) -> Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    loop {
        match (ai.peek(), bi.peek()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(ac), Some(bc)) if ac.is_ascii_digit() && bc.is_ascii_digit() => {
                let mut na: u64 = 0;
                let mut nb: u64 = 0;
                while let Some(c) = ai.peek().copied() {
                    if let Some(d) = c.to_digit(10) {
                        na = na.saturating_mul(10).saturating_add(d as u64);
                        ai.next();
                    } else {
                        break;
                    }
                }
                while let Some(c) = bi.peek().copied() {
                    if let Some(d) = c.to_digit(10) {
                        nb = nb.saturating_mul(10).saturating_add(d as u64);
                        bi.next();
                    } else {
                        break;
                    }
                }
                match na.cmp(&nb) {
                    Ordering::Equal => continue,
                    o => return o,
                }
            }
            (Some(ac), Some(bc)) => {
                let ord = ac.cmp(bc);
                ai.next();
                bi.next();
                if ord != Ordering::Equal {
                    return ord;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_order_basic() {
        let mut v = vec!["page_10.jpg", "page_2.jpg", "page_1.jpg", "page_20.jpg"];
        v.sort_by(|a, b| natord_cmp(a, b));
        assert_eq!(
            v,
            vec!["page_1.jpg", "page_2.jpg", "page_10.jpg", "page_20.jpg"]
        );
    }

    #[test]
    fn natural_order_mixed() {
        let mut v = vec!["ch1_p10.jpg", "ch1_p2.jpg", "ch2_p1.jpg", "ch1_p1.jpg"];
        v.sort_by(|a, b| natord_cmp(a, b));
        assert_eq!(
            v,
            vec!["ch1_p1.jpg", "ch1_p2.jpg", "ch1_p10.jpg", "ch2_p1.jpg"]
        );
    }

    #[test]
    fn is_image_recognises_extensions() {
        assert!(is_image("page_001.jpg"));
        assert!(is_image("PAGE.PNG"));
        assert!(is_image("cover.webp"));
        assert!(!is_image("readme.txt"));
        assert!(!is_image("nodot"));
    }
}
