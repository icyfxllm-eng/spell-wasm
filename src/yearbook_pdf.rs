//! BD-5 CC-YEARBOOK — the deterministic PDF assembler (I3).
//!
//! Raster-page PDF: each spread arrives as one pre-rendered JPEG; this
//! module wraps them in the smallest correct PDF that a printer respects.
//! DETERMINISM RULES, all load-bearing for the golden tests:
//!   * fixed object order (catalog, pages, per-page [page, xobject, content]),
//!   * no /Info dictionary, no /ID, no CreationDate — metadata-free is a
//!     yearbook invariant, not an optimization (acceptance #4),
//!   * integer coordinates only, one fixed generator path.
//! Same pages + same paper => byte-identical output, on any device.

/// Paper in PDF points (1/72"). A4 and US Letter only (readme feature 2).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Paper {
    A4,
    Letter,
}

impl Paper {
    pub fn size(self) -> (u32, u32) {
        match self {
            Paper::A4 => (595, 842),
            Paper::Letter => (612, 792),
        }
    }
    /// Print-at-home margin: 36pt (half inch) all round.
    pub const MARGIN: u32 = 36;
}

pub struct PageJpeg {
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
}

/// Assemble pages into a complete PDF. Pure bytes-in bytes-out.
pub fn assemble(pages: &[PageJpeg], paper: Paper) -> Vec<u8> {
    let (pw, ph) = paper.size();
    let m = Paper::MARGIN;
    let (iw, ih) = (pw - 2 * m, ph - 2 * m);

    // Object numbering: 1 catalog, 2 pages-root, then per page i (0-based):
    // 3+3i page, 4+3i image xobject, 5+3i content stream.
    let n_objs = 2 + pages.len() * 3;
    let mut objs: Vec<Vec<u8>> = Vec::with_capacity(n_objs);

    let kids: Vec<String> = (0..pages.len()).map(|i| format!("{} 0 R", 3 + 3 * i)).collect();
    objs.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objs.push(
        format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), pages.len())
            .into_bytes(),
    );

    for (i, pg) in pages.iter().enumerate() {
        let img_no = 4 + 3 * i;
        let content_no = 5 + 3 * i;
        // Fit the raster inside the margin box, centered, aspect preserved.
        let (bw, bh) = fit(pg.width, pg.height, iw, ih);
        let (bx, by) = (m + (iw - bw) / 2, m + (ih - bh) / 2);
        objs.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {pw} {ph}] \
                 /Resources << /XObject << /Im0 {img_no} 0 R >> >> \
                 /Contents {content_no} 0 R >>"
            )
            .into_bytes(),
        );
        let mut img = format!(
            "<< /Type /XObject /Subtype /Image /Width {} /Height {} \
             /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode \
             /Length {} >>\nstream\n",
            pg.width,
            pg.height,
            pg.bytes.len()
        )
        .into_bytes();
        img.extend_from_slice(&pg.bytes);
        img.extend_from_slice(b"\nendstream");
        objs.push(img);
        let ops = format!("q {bw} 0 0 {bh} {bx} {by} cm /Im0 Do Q");
        objs.push(
            format!("<< /Length {} >>\nstream\n{}\nendstream", ops.len(), ops).into_bytes(),
        );
    }

    // Serialize with a correct xref (offsets computed as we write).
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::with_capacity(n_objs);
    for (i, body) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", n_objs + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    // No /Info, no /ID: the trailer names the catalog and nothing else.
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            n_objs + 1,
            xref_at
        )
        .as_bytes(),
    );
    out
}

fn fit(w: u32, h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    if w == 0 || h == 0 {
        return (max_w, max_h);
    }
    let by_w = (max_w, (h as u64 * max_w as u64 / w as u64) as u32);
    if by_w.1 <= max_h {
        by_w
    } else {
        ((w as u64 * max_h as u64 / h as u64) as u32, max_h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(n: usize) -> Vec<PageJpeg> {
        (0..n)
            .map(|i| PageJpeg {
                width: 1654,
                height: 2339,
                bytes: vec![(i as u8).wrapping_mul(37); 64 + i],
            })
            .collect()
    }

    #[test]
    fn deterministic_and_metadata_free() {
        let a = assemble(&fixture(3), Paper::A4);
        let b = assemble(&fixture(3), Paper::A4);
        assert_eq!(a, b, "same input, same bytes (I3)");
        let text = String::from_utf8_lossy(&a);
        for banned in ["/Info", "/ID", "CreationDate", "Producer", "ModDate"] {
            assert!(!text.contains(banned), "{banned} leaked into the PDF (acceptance #4)");
        }
        assert!(a.starts_with(b"%PDF-1.4"));
        assert!(text.ends_with("%%EOF\n"));
    }

    #[test]
    fn both_papers_and_the_xref_is_honest() {
        for paper in [Paper::A4, Paper::Letter] {
            let pdf = assemble(&fixture(2), paper);
            let text = String::from_utf8_lossy(&pdf);
            // Every advertised offset must point at "N 0 obj".
            let xref_at = text.rfind("xref\n").unwrap();
            // skip "xref", "0 N", and the free-object line; entries follow.
            for (i, line) in text[xref_at..].lines().skip(3).take(8).enumerate() {
                let off: usize = line[..10].parse().unwrap();
                let expect = format!("{} 0 obj", i + 1);
                assert!(
                    text[off..].starts_with(&expect),
                    "xref entry {i} points at {:?}",
                    &text[off..off + 12]
                );
            }
        }
    }

    #[test]
    fn a4_and_letter_differ_only_where_paper_does() {
        let a4 = assemble(&fixture(1), Paper::A4);
        let letter = assemble(&fixture(1), Paper::Letter);
        assert_ne!(a4, letter);
        assert_eq!(
            String::from_utf8_lossy(&a4).matches("stream").count(),
            String::from_utf8_lossy(&letter).matches("stream").count()
        );
    }
}
