// `XmlRenderOptions::track_class_prefix`: where layout put marked elements,
// found in the same pass as the pages.
#![cfg(feature = "html")]

use printpdf::html::{add_xml_to_document, add_xml_to_document_tracked, TrackedBoxes, XmlRenderOptions};
use printpdf::{Color, Op, PdfDocument};

fn render(html: &str, prefix: Option<&str>) -> (PdfDocument, TrackedBoxes) {
    let mut options = XmlRenderOptions::default();
    options.track_class_prefix = prefix.map(str::to_string);
    let mut doc = PdfDocument::new("tracked");
    let boxes = add_xml_to_document_tracked(&mut doc, html, &options).expect("renders");
    (doc, boxes)
}

/// Bounding boxes `[x0, y0, x1, y1]` of every fill in `rgb(r, g, b)` on each page.
fn fills(doc: &PdfDocument, rgb: (u8, u8, u8)) -> Vec<(usize, [f32; 4])> {
    let mut out = Vec::new();
    for (page, p) in doc.pages.iter().enumerate() {
        let mut fill = None;
        for op in &p.ops {
            match op {
                Op::SetFillColor { col: Color::Rgb(c) } => {
                    let ch = |v: f32| (v * 255.0).round() as u8;
                    fill = Some((ch(c.r), ch(c.g), ch(c.b)));
                }
                Op::DrawPolygon { polygon } if fill == Some(rgb) => {
                    let pts: Vec<_> = polygon.rings.iter().flat_map(|r| r.points.iter()).collect();
                    let xs = pts.iter().map(|p| p.p.x.0);
                    let ys = pts.iter().map(|p| p.p.y.0);
                    out.push((
                        page,
                        [
                            xs.clone().fold(f32::MAX, f32::min),
                            ys.clone().fold(f32::MAX, f32::min),
                            xs.fold(f32::MIN, f32::max),
                            ys.fold(f32::MIN, f32::max),
                        ],
                    ));
                }
                _ => {}
            }
        }
    }
    out
}

fn tracked(boxes: &TrackedBoxes, class: &str) -> Vec<(usize, [f32; 4])> {
    boxes
        .get(class)
        .map(|v| {
            v.iter()
                .map(|b| (b.page, [b.x, b.y, b.x + b.width, b.y + b.height]))
                .collect()
        })
        .unwrap_or_default()
}

const HTML: &str = "<html><body>\
    <p>Read <span class=\"t-inline\" style=\"background-color:rgb(250,0,1)\">the inline part</span> here.</p>\
    <div class=\"t-block\" style=\"cursor:pointer;background-color:rgb(250,0,2);padding:12px\"><p>a block</p></div>\
    <p class=\"untracked\">nothing</p>\
    </body></html>";

/// Same box, give or take the rounding a painted polygon's points go through
/// (pt -> mm -> pt).
fn same(a: (usize, [f32; 4]), b: (usize, [f32; 4])) -> bool {
    a.0 == b.0 && a.1.iter().zip(b.1).all(|(x, y)| (x - y).abs() < 0.01)
}

#[test]
fn an_inline_elements_text_runs_are_where_azul_paints_its_background() {
    let (doc, boxes) = render(HTML, Some("t-"));
    // azul's display list paints the inline background over the glyph runs
    // (twice, as it happens); printpdf's text renderer adds one more fill of
    // its own that also covers the run's trailing space.
    let painted = fills(&doc, (250, 0, 1));
    let found = tracked(&boxes, "t-inline");
    assert_eq!(found.len(), 1);
    assert!(same(found[0], painted[0]), "{found:?} vs {painted:?}");
    let union_x1 = painted.iter().map(|(_, b)| b[2]).fold(f32::MIN, f32::max);
    assert!(found[0].1[2] <= union_x1 + 0.01);
}

#[test]
fn an_interactive_blocks_box_is_where_its_background_is_painted() {
    let (doc, boxes) = render(HTML, Some("t-"));
    let painted = fills(&doc, (250, 0, 2));
    assert_eq!(painted.len(), 1);
    let found = tracked(&boxes, "t-block");
    // The border box comes first; the text runs inside lie within it.
    assert!(same(found[0], painted[0]), "{found:?} vs {painted:?}");
    for (page, [x0, y0, x1, y1]) in &found[1..] {
        let [px0, py0, px1, py1] = painted[0].1;
        assert_eq!(*page, painted[0].0);
        assert!(*x0 >= px0 && *y0 >= py0 && *x1 <= px1 && *y1 <= py1);
    }
    assert!(!boxes.contains_key("untracked"));
}

#[test]
fn tracking_changes_nothing_on_the_page() {
    let (tracked_doc, _) = render(HTML, Some("t-"));
    let mut plain = PdfDocument::new("plain");
    add_xml_to_document(&mut plain, HTML, &XmlRenderOptions::default()).expect("renders");
    assert_eq!(tracked_doc.pages.len(), plain.pages.len());
    for (a, b) in tracked_doc.pages.iter().zip(&plain.pages) {
        assert_eq!(a.ops, b.ops);
    }
    // Nothing asked for, nothing reported.
    assert!(render(HTML, None).1.is_empty());
}

#[test]
fn a_box_split_across_pages_is_reported_on_each_page() {
    let mut html = String::from("<html><body>");
    html.push_str("<div class=\"t-long\" style=\"cursor:pointer\">");
    for i in 0..120 {
        html.push_str(&format!("<p>line {i}</p>"));
    }
    html.push_str("</div></body></html>");
    let (doc, boxes) = render(&html, Some("t-"));
    assert!(doc.pages.len() > 1);
    let pages: std::collections::BTreeSet<usize> =
        tracked(&boxes, "t-long").iter().map(|(p, _)| *p).collect();
    assert_eq!(pages.len(), doc.pages.len());
}

