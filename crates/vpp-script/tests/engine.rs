//! Running page scripts: the example sites' own scripts on their pages, and
//! the limits that keep a hostile page from harming the viewer.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

use vpp_dom::{Document, NodeId};
use vpp_format::CodeBin;
use vpp_script::{HostRequest, QuickJsEngine, ScriptEngine};

struct Page {
    document: Rc<RefCell<Document>>,
    engine: QuickJsEngine,
}

impl Page {
    fn new(html: &str) -> Self {
        let document = Rc::new(RefCell::new(vpp_dom::parse_html(html)));
        let engine = QuickJsEngine::new(document.clone()).unwrap();
        Self { document, engine }
    }

    fn run(&mut self, source: &str) {
        self.engine
            .run(&CodeBin::classic("test.js", source))
            .unwrap();
    }

    fn node(&self, id: &str) -> NodeId {
        let document = self.document.borrow();
        document.find_by_id(document.root(), id).unwrap()
    }

    fn text(&self, id: &str) -> String {
        self.document.borrow().text_content(self.node(id))
    }

    fn click(&mut self, id: &str) {
        let node = self.node(id);
        self.engine.dispatch_click(node);
    }
}

fn example(site: &str, page: &str) -> (Page, Vec<CodeBin>) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(site);
    let config = std::fs::read_to_string(root.join("vpp.json")).unwrap();
    let options = vpp_template::TemplateOptions {
        project_root: root.clone(),
        component_aliases: vpp_format::SiteConfig::parse(&config).unwrap().components,
    };
    let expanded =
        vpp_template::expand_page(&root.join("pages").join(format!("{page}.html")), &options)
            .unwrap();
    let document = Rc::new(RefCell::new(expanded.document));
    let engine = QuickJsEngine::new(document.clone()).unwrap();
    let scripts = expanded
        .scripts
        .iter()
        .map(|s| CodeBin::classic(format!("{}.js", s.name), s.source.clone()))
        .collect();
    (Page { document, engine }, scripts)
}

#[test]
fn hello_world_home_counts_clicks() {
    let (mut page, scripts) = example("hello-world", "home");
    for script in &scripts {
        page.engine.run(script).unwrap();
    }
    assert_eq!(
        page.engine.take_requests(),
        [HostRequest::Log("app.js ready on this page".into())]
    );

    page.click("hello");
    assert_eq!(page.text("hello"), "Clicked 1");
    assert_eq!(page.text("count-value"), "1");
    assert_eq!(
        page.engine.take_requests(),
        [
            HostRequest::Invalidate,
            HostRequest::Log("clicked hello count 1".into()),
            HostRequest::Popup("Hello from JavaScript! Click #1".into()),
        ]
    );

    page.click("hello");
    page.click("reset");
    assert_eq!(page.text("hello"), "Click me");
    assert_eq!(page.text("count-value"), "0");
}

#[test]
fn app_window_buttons_reach_the_window() {
    let (mut page, scripts) = example("app-window", "home");
    for script in &scripts {
        page.engine.run(script).unwrap();
    }
    page.engine.take_requests();
    page.click("minimize");
    page.click("maximize");
    page.click("close");
    assert_eq!(
        page.engine.take_requests(),
        [
            HostRequest::Minimize,
            HostRequest::Maximize,
            HostRequest::Close
        ]
    );
}

#[test]
fn clicks_bubble_to_ancestors_with_the_right_target() {
    let mut page = Page::new("<div id=outer><p id=inner><b id=deep>x</b></p></div>");
    page.run(
        "document.getElementById('outer').addEventListener('click', function (e) {
             console.log(e.target.id, this.id, e.type);
         });",
    );
    page.click("deep");
    assert_eq!(
        page.engine.take_requests(),
        [HostRequest::Log("deep outer click".into())]
    );
}

#[test]
fn clicking_text_targets_its_element() {
    let mut page = Page::new("<p id=p>hello</p>");
    page.run(
        "document.getElementById('p').addEventListener('click', e => console.log(e.target.id));",
    );
    let text = page.document.borrow()[page.node("p")].children()[0];
    page.engine.dispatch_click(text);
    assert_eq!(page.engine.take_requests(), [HostRequest::Log("p".into())]);
}

#[test]
fn element_properties() {
    let mut page = Page::new("<section id=s>a <b>b</b></section>");
    page.run(
        "const s = document.getElementById('s');
         console.log(s.id, s.tagName, s.textContent, document.getElementById('none'));
         console.log(s === document.getElementById('s'));",
    );
    assert_eq!(
        page.engine.take_requests(),
        [
            HostRequest::Log("s SECTION a b null".into()),
            HostRequest::Log("true".into()),
        ]
    );
}

#[test]
fn a_listener_that_throws_is_logged_and_the_rest_still_run() {
    let mut page = Page::new("<button id=b>x</button>");
    page.run(
        "const b = document.getElementById('b');
         b.addEventListener('click', () => { throw new Error('boom'); });
         b.addEventListener('click', () => console.log('second ran'));",
    );
    page.click("b");
    let requests = page.engine.take_requests();
    assert!(
        matches!(&requests[0], HostRequest::Log(line) if line.starts_with("uncaught: Error: boom")),
        "{requests:?}"
    );
    assert_eq!(requests[1], HostRequest::Log("second ran".into()));
}

#[test]
fn script_errors_name_the_file() {
    let mut page = Page::new("<p>x</p>");
    let err = page
        .engine
        .run(&CodeBin::classic("app.js", "undefinedFunction();"))
        .unwrap_err();
    assert_eq!(err.filename, "app.js");
    assert!(err.message.contains("undefinedFunction"), "{}", err.message);
    // The page keeps working after a failed script.
    page.run("console.log('still alive')");
    assert!(
        page.engine
            .take_requests()
            .contains(&HostRequest::Log("still alive".into()))
    );
}

#[test]
fn a_runaway_script_is_stopped() {
    // INTENTIONAL: hostile script. An endless loop must hit the time limit, not freeze the viewer.
    let document = Rc::new(RefCell::new(vpp_dom::parse_html("<p>x</p>")));
    let mut engine = QuickJsEngine::with_time_limit(document, Duration::from_millis(200)).unwrap();
    let started = Instant::now();
    let err = engine
        .run(&CodeBin::classic("loop.js", "while (true) {}"))
        .unwrap_err();
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(err.message.contains("interrupted"), "{}", err.message);
    // The limit is per run: the next script has its full time again.
    engine.run(&CodeBin::classic("ok.js", "1 + 1")).unwrap();
}

#[test]
fn a_runaway_listener_is_stopped_and_logged() {
    // INTENTIONAL: hostile script. An endless loop in a click handler must not freeze the viewer.
    let document = Rc::new(RefCell::new(vpp_dom::parse_html("<button id=b>x</button>")));
    let mut engine =
        QuickJsEngine::with_time_limit(document.clone(), Duration::from_millis(200)).unwrap();
    engine
        .run(&CodeBin::classic(
            "t.js",
            "document.getElementById('b').addEventListener('click', () => { while (true) {} });",
        ))
        .unwrap();
    let b = {
        let d = document.borrow();
        d.find_by_id(d.root(), "b").unwrap()
    };
    engine.dispatch_click(b);
    let requests = engine.take_requests();
    assert!(
        matches!(&requests[0], HostRequest::Log(line) if line.contains("interrupted")),
        "{requests:?}"
    );
}

#[test]
fn memory_is_limited() {
    // INTENTIONAL: hostile script. Allocating without end must fail inside the engine.
    let mut page = Page::new("<p>x</p>");
    let err = page
        .engine
        .run(&CodeBin::classic(
            "hog.js",
            "let a = []; for (;;) a.push(new Array(1e6).fill(1));",
        ))
        .unwrap_err();
    assert!(
        err.message.to_lowercase().contains("memory"),
        "{}",
        err.message
    );
}

#[test]
fn forged_or_stale_elements_are_refused() {
    let mut page = Page::new("<div id=d><b id=gone>x</b></div>");
    page.run(
        "const gone = document.getElementById('gone');
         document.getElementById('d').textContent = 'replaced';
         try { gone.textContent; } catch (e) { console.log(e.name, e.message); }
         const Element = gone.constructor;
         try { new Element(9999).id; } catch (e) { console.log(e.name, e.message); }",
    );
    let requests = page.engine.take_requests();
    assert!(
        requests.contains(&HostRequest::Log(
            "TypeError the element was removed from the page".into()
        )),
        "{requests:?}"
    );
    assert!(
        requests.contains(&HostRequest::Log(
            "TypeError not an element of this page".into()
        )),
        "{requests:?}"
    );
}

#[test]
fn promises_run_after_the_script() {
    let mut page = Page::new("<p>x</p>");
    page.run("Promise.resolve().then(() => console.log('later')); console.log('now');");
    assert_eq!(
        page.engine.take_requests(),
        [
            HostRequest::Log("now".into()),
            HostRequest::Log("later".into())
        ]
    );
}
