// `hyphens: auto` with and without the `text_layout_hyphenation` feature:
// a build without the dictionaries lays the text out unhyphenated, never fails.
#![cfg(feature = "html")]

use printpdf::html::{add_xml_to_document, XmlRenderOptions};
use printpdf::{Op, PdfDocument};

const HTML: &str = "<html><body><p lang=\"en\" style=\"width:70px;hyphens:auto\">\
    internationalization characteristically</p></body></html>";

fn text_ops() -> usize {
    let mut doc = PdfDocument::new("hyphens");
    add_xml_to_document(&mut doc, HTML, &XmlRenderOptions::default()).expect("renders");
    doc.pages
        .iter()
        .flat_map(|p| p.ops.iter())
        .filter(|op| matches!(op, Op::ShowText { .. }))
        .count()
}

#[test]
fn hyphens_auto_renders_with_or_without_dictionaries() {
    assert!(text_ops() > 0, "the paragraph's text is drawn");
}
