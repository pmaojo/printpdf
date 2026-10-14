// HTML layout — only compiled with the `html` feature.
#![cfg(feature = "html")]

use std::collections::BTreeMap;
use printpdf::{GeneratePdfOptions, PdfDocument, PdfSaveOptions, PdfWarnMsg};

fn render() -> Vec<u8> {
    // Translucent fills (ExtGState) and a gradient (shading) are the two
    // resource kinds the HTML bridge names itself.
    let html = "<!DOCTYPE html><html><body>\
        <div style=\"width:100px;height:40px;background:rgba(255,0,0,0.5)\">a</div>\
        <div style=\"width:100px;height:40px;background:rgba(0,0,255,0.5)\">b</div>\
        <div style=\"width:100px;height:40px;background:linear-gradient(to right,#f00,#00f)\">c</div>\
        </body></html>";
    let mut warnings: Vec<PdfWarnMsg> = Vec::new();
    let doc = PdfDocument::from_html(
        html,
        &BTreeMap::default(),
        &BTreeMap::default(),
        &GeneratePdfOptions::default(),
        &mut warnings,
    )
    .unwrap();
    doc.save(&PdfSaveOptions::default(), &mut warnings)
}

#[test]
fn rendering_the_same_html_twice_gives_identical_bytes_modulo_dates() {
    // Resource names must not be random. Dates/IDs in the trailer are the
    // document's business, so compare the resource-name set instead.
    let names = |bytes: &[u8]| {
        let text = String::from_utf8_lossy(bytes).into_owned();
        let mut out: Vec<String> = text
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|w| w.len() == 32 && (w.starts_with('G') || w.starts_with('S')))
            .map(str::to_string)
            .collect();
        out.sort();
        out.dedup();
        out
    };
    let (a, b) = (names(&render()), names(&render()));
    assert!(!a.is_empty(), "expected synthesized ExtGState/shading names");
    assert_eq!(a, b);
}
