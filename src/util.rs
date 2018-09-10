use errors::OResult;
use std::cell::{Cell, RefCell};
use std::io::{ BufReader, BufWriter, Read, Write };
use std::path::Path;
use std::fs::{ self, File };
use std::rc::Rc;
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{Color, FontStyle, Style, ThemeSet};
use html5ever::rcdom::{RcDom, Node, NodeData, Handle};
use html5ever::LocalName;
use html5ever::interface::QualName;
use html5ever::tree_builder::Attribute;
use html5ever_ext::{RcDomExt, Minify};
use regex::Regex;

pub fn read_file<P: AsRef<Path>>(path: P) -> OResult<String> {
    let path = path.as_ref();
    let mut br = BufReader::new(File::open(path)?);
    let mut result = String::new();
    br.read_to_string(&mut result)?;
    Ok(result)
}

pub fn write_minified_html<P, B>(path: P, content: B) -> OResult<()>
where P: AsRef<Path>,
      B: AsRef<[u8]>
{
    let mut dom = RcDom::from_bytes(content.as_ref());
    inspect_dom(&mut dom);
    dom.minify_to_file_path(false, path)?;
    Ok(())
}

pub fn write_file<P, B>(path: P, content: B) -> OResult<()>
where P: AsRef<Path>,
      B: AsRef<[u8]>
{
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut br = BufWriter::new(File::create(path)?);
    br.write_all(content.as_ref())?;
    Ok(())
}

fn inspect_dom(dom: &mut RcDom) {
    inspect_node(&mut dom.document);
}

fn inspect_node(node: &mut Handle) {
    lazy_static! {
        static ref RE: Regex = Regex::new("language-([a-z]+)").unwrap();
    }
    if let NodeData::Element { ref name, ref attrs, .. } = node.data {
        if &name.local == "code" {
            if let Some(attr) =  attrs.borrow().iter().find(|attr| &attr.name.local == "class") {
                if let Some(ref language_match) = RE.find(&*attr.value) {
                    let new = text_to_highlighted(
                        language_match.as_str(),
                        &node.children.borrow()
                    );
                    node.children.replace(new);
                    return;
                }
            }
        }
    }

    for child in node.children.borrow_mut().iter_mut() {
        inspect_node(child)
    }
}

fn text_to_highlighted(language: &str, children: &[Handle]) -> Vec<Handle> {
    if let [node] = children {
        if let NodeData::Text { ref contents } = node.data {
            let parent = node.clone();
            return highlight_code(language, &contents.borrow(), parent)
        }
    }
    error!("Can only highlight code blocks with single, text child node");
    return children.to_owned()
}

fn highlight_code(language: &str, code: &str, parent: Handle) -> Vec<Handle> {
    let ss = SyntaxSet::load_defaults_nonewlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ss.find_syntax_by_token(language)
        .unwrap_or(ss.find_syntax_plain_text());
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    let mut children: Vec<Handle> = Vec::new();
    for ranges in code.lines().map(|line| h.highlight(line)) {
        for (style, text) in ranges {
            children.push(style_to_node(style, text, parent.clone()))
        }
    }
    children
}

fn style_to_node(style: Style, text: &str, parent: Handle) -> Handle {
    let parent = Cell::new(Some(Rc::downgrade(&parent)));
    let text = Rc::new(Node {
        parent: Cell::new(None),
        children: RefCell::new(vec![]),
        data: NodeData::Text {
            contents: RefCell::new(text.to_owned().into())
        }
    });
    let style = Attribute {
        name: QualName { prefix: None, ns: ns!(), local: LocalName::from("style") },
        value: style_to_attr(style).into()
    };
    let data = NodeData::Element {
        name: QualName {
            prefix: None,
            ns: ns!(),
            local: LocalName::from("span"),
        },
        attrs: RefCell::new(vec![style]),
        template_contents: None,
        mathml_annotation_xml_integration_point: false,
    };
    let res = Rc::new(Node { parent, data, children: RefCell::new(vec![text.clone()]) });
    let cloned = res.clone();
    let parent_pointer = Some(Rc::downgrade(&cloned));
    text.parent.replace(parent_pointer);
    res
}

fn style_to_attr(style: Style) -> String {
    let mut res = String::new();
    if style.font_style.contains(FontStyle::UNDERLINE) {
        res.push_str("text-decoration:underline;");
    }
    if style.font_style.contains(FontStyle::BOLD) {
        res.push_str("font-weight:bold;");
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        res.push_str("font-style:italic");
    }
    res.push_str("color:");
    write_css_color(&mut res, style.foreground);
    res
}

// shamelssly stolen from syntect source
fn write_css_color(s: &mut String, c: Color) {
    use std::fmt::Write;
    if c.a != 0xFF {
        write!(s,"#{:02x}{:02x}{:02x}{:02x}",c.r,c.g,c.b,c.a).unwrap();
    } else {
        write!(s,"#{:02x}{:02x}{:02x}",c.r,c.g,c.b).unwrap();
    }
}
