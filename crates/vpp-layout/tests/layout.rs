//! Layout of small pages with [`FixedMeasure`]: every character is 8 px wide
//! at 16 px, lines are 20 px tall, and the baseline is 16 px down, so the
//! expected positions can be worked out by hand.

use vpp_dom::Document;
use vpp_layout::{FixedMeasure, FragmentContent, LayoutBox, layout_document};
use vpp_style::parse_stylesheet;

/// A laid-out page and its document.
struct Page {
    document: Document,
    root: LayoutBox,
}

fn lay_out(css: &str, html: &str, width: f32) -> Page {
    let document = vpp_dom::parse_html(html);
    let sheets = [parse_stylesheet(css)];
    let root = layout_document(&document, &sheets, &FixedMeasure, width).unwrap();
    Page { document, root }
}

impl Page {
    /// The box of the element with id `id`.
    fn find(&self, id: &str) -> &LayoutBox {
        self.try_find(id)
            .unwrap_or_else(|| panic!("no box for #{id}"))
    }

    fn try_find(&self, id: &str) -> Option<&LayoutBox> {
        let node = self.document.find_by_id(self.document.root(), id)?;
        find_node(&self.root, node)
    }

    /// The words on each line inside the box of `id`, in order.
    fn lines(&self, id: &str) -> Vec<Vec<String>> {
        let mut out = Vec::new();
        collect_lines(self.find(id), &mut out);
        out
    }
}

fn find_node(b: &LayoutBox, node: vpp_dom::NodeId) -> Option<&LayoutBox> {
    if b.node == Some(node) {
        return Some(b);
    }
    b.children.iter().find_map(|c| find_node(c, node))
}

fn collect_lines(b: &LayoutBox, out: &mut Vec<Vec<String>>) {
    for line in &b.lines {
        out.push(
            line.fragments
                .iter()
                .filter_map(|f| match &f.content {
                    FragmentContent::Text { text, .. } => Some(text.clone()),
                    FragmentContent::Box(_) => None,
                })
                .collect(),
        );
    }
    for child in &b.children {
        collect_lines(child, out);
    }
}

const RESET: &str = "body { margin: 0 } p { margin: 0 }";

#[test]
fn blocks_stack_and_fill_the_width() {
    let page = lay_out("", "<p id=a>Hi</p><p id=b>Yo</p>", 400.0);
    let (a, b) = (page.find("a"), page.find("b"));
    // body's 16 px margin on each side.
    assert_eq!((a.frame.x, a.frame.w), (16.0, 368.0));
    assert_eq!(a.frame.h, 20.0);
    // Adjacent 16 px margins collapse into one.
    assert_eq!(b.frame.y - (a.frame.y + a.frame.h), 16.0);
}

#[test]
fn text_wraps_at_the_available_width() {
    // "aaaa bbbb" is 32 + 8 + 32 = 72 px; "cccc" does not fit after it in 80.
    let page = lay_out(RESET, "<p id=t>aaaa bbbb cccc</p>", 80.0);
    assert_eq!(page.lines("t"), [vec!["aaaa", "bbbb"], vec!["cccc"]]);
    assert_eq!(page.find("t").frame.h, 40.0);
}

#[test]
fn whitespace_collapses_between_words_and_elements() {
    let page = lay_out(RESET, "<p id=t>  one\n\n   <b>two</b>three  </p>", 400.0);
    assert_eq!(page.lines("t"), [vec!["one", "two", "three"]]);
    let line = &page.find("t").children[0].lines[0];
    // "one" 24 px, space 8, "two" 24 px, no space, "three".
    assert_eq!(line.fragments[1].rect.x, 32.0);
    assert_eq!(line.fragments[2].rect.x, 56.0);
}

#[test]
fn text_align_center_and_right() {
    let page = lay_out(
        &format!("{RESET} #c {{ text-align: center }} #r {{ text-align: right }}"),
        "<p id=c>ab</p><p id=r>ab</p>",
        100.0,
    );
    assert_eq!(
        page.find("c").children[0].lines[0].fragments[0].rect.x,
        42.0
    );
    assert_eq!(
        page.find("r").children[0].lines[0].fragments[0].rect.x,
        84.0
    );
}

#[test]
fn inline_block_shrinks_to_fit_and_sits_on_the_baseline() {
    let page = lay_out(RESET, "<p id=p>Go <button id=b>OK</button></p>", 400.0);
    let button = page.find("b");
    // "OK" is 16 px, plus 24 px padding each side; 20 px line plus 12 px padding above and below.
    assert_eq!((button.frame.w, button.frame.h), (64.0, 44.0));
    // After "Go" (16 px) and a space (8 px).
    assert_eq!(button.frame.x, 24.0);
    assert_eq!(button.baseline, 28.0);
    // The word and the button's text share a baseline.
    let line = &page.find("p").children[0].lines[0];
    let word = &line.fragments[0];
    assert_eq!(
        word.rect.y + word.baseline,
        button.frame.y + button.baseline
    );
}

#[test]
fn max_width_with_auto_margins_centres_a_block() {
    let page = lay_out(
        &format!("{RESET} .c {{ max-width: 100px; margin: 0 auto }}"),
        "<div id=c class=c>x</div>",
        400.0,
    );
    let c = page.find("c");
    assert_eq!((c.frame.x, c.frame.w), (150.0, 100.0));
}

#[test]
fn percentage_and_explicit_sizes() {
    let page = lay_out(
        &format!("{RESET} #a {{ width: 50%; height: 30px; padding: 5px }}"),
        "<div id=a></div>",
        400.0,
    );
    let a = page.find("a");
    // Content-box sizes: padding is added around them.
    assert_eq!((a.frame.w, a.frame.h), (210.0, 40.0));
}

#[test]
fn flex_row_grows_items_with_gaps() {
    let page = lay_out(
        &format!("{RESET} .f {{ display: flex; gap: 10px }} .f div {{ flex: 1 }}"),
        "<div class=f><div id=a></div><div id=b></div><div id=c></div></div>",
        400.0,
    );
    let (a, b, c) = (page.find("a"), page.find("b"), page.find("c"));
    let third = (400.0 - 20.0) / 3.0;
    assert!((a.frame.w - third).abs() < 0.01);
    assert!((b.frame.x - (third + 10.0)).abs() < 0.01);
    assert!((c.frame.x + c.frame.w - 400.0).abs() < 0.01);
}

#[test]
fn flex_justify_and_align() {
    let css = format!(
        "{RESET} .f {{ display: flex; justify-content: space-between; align-items: center }} \
         #a {{ width: 50px; height: 20px }} #b {{ width: 50px; height: 60px }}"
    );
    let page = lay_out(
        &css,
        "<div class=f><div id=a></div><div id=b></div></div>",
        400.0,
    );
    let (a, b) = (page.find("a"), page.find("b"));
    assert_eq!(a.frame.x, 0.0);
    assert_eq!(b.frame.x, 350.0);
    // Centred on the 60 px line.
    assert_eq!(a.frame.y - b.frame.y, 20.0);
}

#[test]
fn flex_column_stretches_items() {
    let page = lay_out(
        &format!("{RESET} .f {{ display: flex; flex-direction: column; gap: 4px }}"),
        "<div class=f><div id=a>one</div><div id=b>two</div></div>",
        300.0,
    );
    let (a, b) = (page.find("a"), page.find("b"));
    assert_eq!(a.frame.w, 300.0);
    assert_eq!(b.frame.y, a.frame.y + 20.0 + 4.0);
}

#[test]
fn text_directly_inside_a_flex_container_is_laid_out() {
    let page = lay_out(
        &format!("{RESET} #f {{ display: flex }}"),
        "<div id=f>hello</div>",
        300.0,
    );
    assert_eq!(page.lines("f"), [vec!["hello"]]);
}

#[test]
fn hidden_elements_have_no_box_and_whitespace_makes_none() {
    let page = lay_out(
        RESET,
        "<div id=d>\n  <p id=a style='display:none'>x</p>\n  <p id=b>y</p>\n</div>",
        300.0,
    );
    assert!(page.try_find("a").is_none());
    let d = page.find("d");
    assert_eq!(d.children.len(), 1);
    assert_eq!(
        d.children[0].node,
        page.document.find_by_id(page.document.root(), "b")
    );
}

#[test]
fn svg_is_sized_by_its_attributes() {
    let page = lay_out(RESET, "<p><svg id=s width=24 height=12></svg></p>", 300.0);
    let s = page.find("s");
    assert_eq!((s.frame.w, s.frame.h), (24.0, 12.0));
    // A replaced element sits on the baseline with its bottom edge.
    assert_eq!(s.baseline, 12.0);
}

#[test]
fn hit_test_finds_the_deepest_element() {
    let page = lay_out(RESET, "<p>Go <button id=b>OK</button></p>", 400.0);
    let b = page.find("b");
    let hit = page
        .root
        .hit_test(b.frame.x + 5.0, b.frame.y + 5.0)
        .unwrap();
    assert_eq!(
        hit.node,
        page.document.find_by_id(page.document.root(), "b")
    );
}

#[test]
fn every_example_page_lays_out() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    for site in ["hello-world", "app-window"] {
        for page in ["home", "about"] {
            let options = vpp_template::TemplateOptions {
                project_root: root.join(site),
                component_aliases: [("stat-card".to_string(), "/components/stat-card".to_string())]
                    .into(),
            };
            let file = root.join(site).join("pages").join(format!("{page}.html"));
            let expanded = vpp_template::expand_page(&file, &options).unwrap();
            let sheets: Vec<_> = expanded.styles.into_iter().map(|s| s.sheet).collect();
            let laid = layout_document(&expanded.document, &sheets, &FixedMeasure, 800.0).unwrap();
            assert!(
                laid.frame.h > 100.0,
                "{site}/{page} is {} px tall",
                laid.frame.h
            );
        }
    }
}
