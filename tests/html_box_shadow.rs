// HTML layout — only compiled with the `html` feature.
#![cfg(feature = "html")]

use std::collections::BTreeMap;
use printpdf::{GeneratePdfOptions, Op, PdfDocument};

fn polygon_count(body: &str) -> usize {
    let html = format!(
        "<!DOCTYPE html><html><body>\
         <div style=\"width:100px;height:40px;background:#eeeeee;{body}\">x</div>\
         </body></html>"
    );
    let options = GeneratePdfOptions::default();
    let mut warnings = Vec::new();
    let doc = PdfDocument::from_html(
        &html,
        &BTreeMap::default(),
        &BTreeMap::default(),
        &options,
        &mut warnings,
    )
    .unwrap();
    doc.pages
        .iter()
        .flat_map(|p| p.ops.iter())
        .filter(|op| matches!(op, Op::DrawPolygon { .. }))
        .count()
}

#[test]
fn box_shadow_is_painted() {
    let flat = polygon_count("");
    let hard = polygon_count("box-shadow: 4px 4px 0 #000000;");
    let soft = polygon_count("box-shadow: 0 4px 8px rgba(0,0,0,0.5);");
    assert!(hard > flat, "a hard shadow draws one extra shape ({hard} vs {flat})");
    assert!(soft > hard, "a blurred shadow draws stacked layers ({soft} vs {hard})");
}

#[test]
fn inset_box_shadow_draws_nothing_extra() {
    let flat = polygon_count("");
    let inset = polygon_count("box-shadow: inset 0 2px 4px #000000;");
    assert_eq!(inset, flat);
}
