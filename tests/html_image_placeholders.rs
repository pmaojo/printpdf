// `XmlRenderOptions::image_placeholders`: an `<img>` laid out by size alone.
#![cfg(all(feature = "html", feature = "jpeg"))]

use std::collections::BTreeMap;

use printpdf::html::{add_xml_to_document, XmlRenderOptions};
use printpdf::{PdfDocument, PdfSaveOptions, XObject};

const CAT_JPG: &[u8] = include_bytes!("../examples/assets/img/cat.jpg");

// `width` only: the height comes from the intrinsic aspect ratio, so layout
// really depends on the size it is given.
const HTML: &str = "<html><body><p>before</p>\
    <img src=\"cat.jpg\" style=\"width:200px\">\
    <p>after</p></body></html>";

fn render(options: &XmlRenderOptions) -> PdfDocument {
    let mut doc = PdfDocument::new("placeholders");
    add_xml_to_document(&mut doc, HTML, options).expect("renders");
    doc
}

fn cat_size() -> (u32, u32) {
    let mut warnings = Vec::new();
    let raw = printpdf::RawImage::decode_from_bytes(CAT_JPG, &mut warnings).expect("decodes");
    (raw.width as u32, raw.height as u32)
}

#[test]
fn a_placeholder_lays_out_and_places_exactly_like_the_real_image() {
    let mut real = XmlRenderOptions::default();
    real.images.insert("cat.jpg".to_string(), CAT_JPG.to_vec());
    let mut sized = XmlRenderOptions::default();
    sized.image_placeholders = BTreeMap::from([("cat.jpg".to_string(), cat_size())]);

    // Control: the size really drives layout (a wider aspect, a shorter box).
    let (w, h) = cat_size();
    let mut wrong = XmlRenderOptions::default();
    wrong.image_placeholders = BTreeMap::from([("cat.jpg".to_string(), (w * 2, h))]);
    assert_ne!(render(&wrong).pages[0].ops, render(&sized).pages[0].ops);

    let (real, sized) = (render(&real), render(&sized));
    // Same page operations: same layout, same placement transform.
    assert_eq!(real.pages.len(), sized.pages.len());
    for (a, b) in real.pages.iter().zip(&sized.pages) {
        assert_eq!(a.ops, b.ops);
    }
    // ...and the same `cm` once saved, because the stand-in declares the size.
    let mut warnings = Vec::new();
    let opts = PdfSaveOptions::default();
    let cm = |bytes: Vec<u8>| -> Vec<String> {
        let doc = lopdf::Document::load_mem(&bytes).expect("valid PDF");
        let page = *doc.get_pages().get(&1).expect("one page");
        let content = doc.get_and_decode_page_content(page).expect("content");
        content
            .operations
            .iter()
            .filter(|op| op.operator == "cm")
            .map(|op| format!("{:?}", op.operands))
            .collect()
    };
    assert_eq!(cm(real.save(&opts, &mut warnings)), cm(sized.save(&opts, &mut warnings)));
}

#[test]
fn a_placeholder_is_a_one_pixel_stand_in_that_declares_its_size() {
    let mut options = XmlRenderOptions::default();
    options.image_placeholders = BTreeMap::from([("cat.jpg".to_string(), (800, 600))]);
    let doc = render(&options);
    let stand_in = doc
        .resources
        .xobjects
        .map
        .get(&printpdf::html::bridge::image_xobject_id("cat.jpg"))
        .expect("registered under the bridge's id");
    let XObject::External(external) = stand_in else {
        panic!("expected a stand-in, got {stand_in:?}");
    };
    assert_eq!((external.width, external.height), (Some(printpdf::Px(800)), Some(printpdf::Px(600))));
    assert_eq!(external.stream.content, vec![0]);
    // The saved document is a valid PDF with a 1x1 image in that slot.
    let mut warnings = Vec::new();
    let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
    let pdf = lopdf::Document::load_mem(&bytes).expect("valid PDF");
    let images: Vec<_> = pdf
        .objects
        .values()
        .filter_map(|o| o.as_stream().ok())
        .filter(|s| s.dict.get(b"Subtype").and_then(|v| v.as_name()).ok() == Some(&b"Image"[..]))
        .collect();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].dict.get(b"Width").and_then(|v| v.as_i64()).ok(), Some(1));
}

#[test]
fn real_bytes_win_over_a_placeholder_of_the_same_id() {
    let mut options = XmlRenderOptions::default();
    options.images.insert("cat.jpg".to_string(), CAT_JPG.to_vec());
    options.image_placeholders = BTreeMap::from([("cat.jpg".to_string(), (1, 1))]);
    let doc = render(&options);
    let image = doc
        .resources
        .xobjects
        .map
        .get(&printpdf::html::bridge::image_xobject_id("cat.jpg"))
        .expect("registered");
    assert!(matches!(image, XObject::Image(_)));
}
