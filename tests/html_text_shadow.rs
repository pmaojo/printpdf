// HTML layout — only compiled with the `html` feature.
#![cfg(feature = "html")]

use std::collections::BTreeMap;
use printpdf::{GeneratePdfOptions, Op, PdfDocument};

fn show_text_count(style: &str) -> usize {
    let html = format!("<!DOCTYPE html><html><body><p style=\"{style}\">Hello</p></body></html>");
    let mut warnings = Vec::new();
    let doc = PdfDocument::from_html(
        &html,
        &BTreeMap::default(),
        &BTreeMap::default(),
        &GeneratePdfOptions::default(),
        &mut warnings,
    )
    .unwrap();
    doc.pages
        .iter()
        .flat_map(|p| p.ops.iter())
        .filter(|op| matches!(op, Op::ShowText { .. }))
        .count()
}

#[test]
fn text_shadow_draws_the_text_again_offset() {
    let plain = show_text_count("");
    let hard = show_text_count("text-shadow: 2px 2px 0 #ff0000;");
    assert!(plain > 0);
    assert_eq!(hard, plain * 2, "one shadow copy plus the text itself");
}

#[test]
fn a_blurred_text_shadow_spreads_several_translucent_copies() {
    let plain = show_text_count("");
    let soft = show_text_count("text-shadow: 0 2px 6px rgba(0,0,0,0.5);");
    assert_eq!(soft, plain * 6, "five taps plus the text");
}

#[test]
fn text_without_a_shadow_is_untouched() {
    assert_eq!(show_text_count("color: #112233;"), show_text_count(""));
}

