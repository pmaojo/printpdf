// HTML layout — only compiled with the `html` feature.
#![cfg(feature = "html")]

use std::collections::BTreeMap;
use printpdf::{Base64OrRaw, GeneratePdfOptions, Op, PdfDocument};

fn png(w: u32, h: u32) -> Vec<u8> {
    // A minimal valid RGBA PNG, built by hand: signature + IHDR + IDAT + IEND.
    fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = (data.len() as u32).to_be_bytes().to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let mut crc_input = kind.to_vec();
        crc_input.extend_from_slice(data);
        let mut crc = 0xffff_ffffu32;
        for b in crc_input {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 != 0 { 0xedb8_8320 ^ (crc >> 1) } else { crc >> 1 };
            }
        }
        out.extend_from_slice(&(!crc).to_be_bytes());
        out
    }
    fn adler(data: &[u8]) -> u32 {
        let (mut a, mut b) = (1u32, 0u32);
        for d in data {
            a = (a + u32::from(*d)) % 65521;
            b = (b + a) % 65521;
        }
        (b << 16) | a
    }
    let mut raw = Vec::new();
    for _ in 0..h {
        raw.push(0u8);
        raw.extend(std::iter::repeat([200u8, 50, 50, 255]).take(w as usize).flatten());
    }
    // zlib stream of stored (uncompressed) deflate blocks.
    let mut z = vec![0x78, 0x01];
    let mut chunks = raw.chunks(65535).peekable();
    while let Some(c) = chunks.next() {
        z.push(u8::from(chunks.peek().is_none()));
        z.extend_from_slice(&(c.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(c.len() as u16)).to_le_bytes());
        z.extend_from_slice(c);
    }
    z.extend_from_slice(&adler(&raw).to_be_bytes());
    let mut ihdr = w.to_be_bytes().to_vec();
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    out.extend(chunk(b"IHDR", &ihdr));
    out.extend(chunk(b"IDAT", &z));
    out.extend(chunk(b"IEND", &[]));
    out
}

/// The (width, height) in points of the first image placement.
fn first_image_scale(img_tag: &str) -> Option<(f32, f32)> {
    let html = format!("<!DOCTYPE html><html><body>{img_tag}</body></html>");
    let mut images = BTreeMap::new();
    images.insert("a.png".to_string(), Base64OrRaw::Raw(png(80, 40)));
    let mut warnings = Vec::new();
    let doc = PdfDocument::from_html(
        &html,
        &images,
        &BTreeMap::default(),
        &GeneratePdfOptions::default(),
        &mut warnings,
    )
    .ok()?;
    doc.pages.iter().flat_map(|p| p.ops.iter()).find_map(|op| match op {
        Op::UseXobject { transform, .. } => {
            Some((transform.scale_x?, transform.scale_y?))
        }
        _ => None,
    })
}

// `Op::UseXobject` scales the image's own pixel grid, so scale_x == scale_y
// means the box has the image's own aspect ratio (80:40 here).

#[test]
fn an_image_with_no_css_size_gets_its_intrinsic_size() {
    let (sx, sy) = first_image_scale(r#"<img src="a.png">"#).expect("image placed");
    assert!(sx > 0.0 && sy > 0.0, "image has a box ({sx} x {sy})");
    assert!((sx - sy).abs() < 0.01, "drawn at its own aspect ratio ({sx} x {sy})");
}

#[test]
fn an_explicit_css_box_still_wins_over_the_intrinsic_size() {
    // A 100x100 box for an 80x40 image: stretched 1.25x wide, 2.5x tall.
    let (sx, sy) = first_image_scale(r#"<img src="a.png" style="width:100px;height:100px">"#)
        .expect("image placed");
    assert!((sy / sx - 2.0).abs() < 0.01, "CSS box overrides intrinsic ({sx} x {sy})");
}

#[test]
fn width_and_height_attributes_the_author_wrote_are_left_alone() {
    let (sx, sy) = first_image_scale(r#"<img src="a.png" width="100" height="100">"#)
        .expect("image placed");
    assert!((sy / sx - 2.0).abs() < 0.01, "author attributes respected ({sx} x {sy})");
}

#[test]
fn the_test_png_decodes() {
    let mut w = Vec::new();
    let r = printpdf::RawImage::decode_from_bytes(&png(80, 40), &mut w);
    assert!(r.is_ok(), "{:?}", r.err());
}

/// The layout engine sees the decoded size: with no CSS size, the image's box
/// is its own pixel size (it used to be a 0x0 placeholder that happened to be
/// scaled up when drawn).
#[test]
fn layout_gets_the_decoded_size_as_the_images_intrinsic_size() {
    use printpdf::html::{xml_to_pdf_pages_debug, XmlRenderOptions};
    let mut options = XmlRenderOptions::default();
    options.images.insert("a.png".to_string(), png(80, 40));
    let (_, _, debug, _) = xml_to_pdf_pages_debug(
        r#"<html><body><img src="a.png"></body></html>"#,
        &options,
    )
    .expect("renders");
    let layout = debug.display_list_debug.join("\n");
    assert!(layout.contains("size=Some(80x40)"), "{layout}");
}
