use errors::OResult;
use html5ever::interface::QualName;
use html5ever::rcdom::{Handle, Node, NodeData, RcDom};
use html5ever::tree_builder::Attribute;
use html5ever::LocalName;
use html5ever_ext::RcDomExt;
use html5ever_ext::UltraMinifyingHtmlSerializer;
use regex::Regex;
use std::cell::{Cell, RefCell};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::rc::Rc;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, FontStyle, Style, ThemeSet};
use syntect::parsing::SyntaxSet;

pub fn read_file<P: AsRef<Path>>(path: P) -> OResult<String> {
    let bytes = read_bytes(path)?;
    let string = String::from_utf8(bytes)?;
    Ok(string)
}

pub fn read_bytes<P: AsRef<Path>>(path: P) -> OResult<Vec<u8>> {
    let path = path.as_ref();
    let mut br = BufReader::new(File::open(path)?);
    let mut result = Vec::new();
    br.read_to_end(&mut result)?;
    Ok(result)
}

pub fn write_minified_html<P, B>(path: P, content: B) -> OResult<()>
where
    P: AsRef<Path>,
    B: AsRef<[u8]>,
{
    let f = File::create(path)?;
    let bw = BufWriter::new(f);
    let mut dom = RcDom::from_bytes(content.as_ref());
    inspect_dom(&mut dom);
    let mut mini = UltraMinifyingHtmlSerializer::new(false, false, false, bw);
    mini.serialize_rc_dom(&dom, true)?;
    Ok(())
}

pub fn write_file<P, B>(path: P, content: B) -> OResult<()>
where
    P: AsRef<Path>,
    B: AsRef<[u8]>,
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
    let re = Regex::new("language-([a-z]+)").unwrap();
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    inspect_node(&mut dom.document, &re, &ss, &ts);
}

fn inspect_node(node: &mut Handle, re: &Regex, ss: &SyntaxSet, ts: &ThemeSet) {
    if let NodeData::Element {
        ref name,
        ref attrs,
        ..
    } = node.data
    {
        if &name.local == "code" {
            if let Some(attr) = attrs
                .borrow()
                .iter()
                .find(|attr| &attr.name.local == "class")
            {
                if let Some(ref language_match) = re.captures(&*attr.value) {
                    let syntax = ss
                        .find_syntax_by_token(language_match.get(1).unwrap().as_str())
                        .unwrap_or(ss.find_syntax_plain_text());
                    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
                    let new = text_to_highlighted(&node.children.borrow(), h);
                    node.children.replace(new);
                    return;
                }
            }
        }
    }

    for child in node.children.borrow_mut().iter_mut() {
        inspect_node(child, re, ss, ts)
    }
}

fn text_to_highlighted(children: &[Handle], h: HighlightLines) -> Vec<Handle> {
    if let [node] = children {
        if let NodeData::Text { ref contents } = node.data {
            let parent = node.clone();
            return highlight_code(&contents.borrow(), parent, h);
        }
    }
    error!("Can only highlight code blocks with single, text child node");
    return children.to_owned();
}

fn highlight_code(code: &str, parent: Handle, mut h: HighlightLines) -> Vec<Handle> {
    let mut children: Vec<Handle> = Vec::new();
    for line in code.lines() {
        let ranges = h.highlight(line);
        for &(style, text) in ranges.iter() {
            if text.trim().is_empty() {
                children.push(text_to_node(text, parent.clone()));
            } else {
                children.push(style_to_node(style, text, parent.clone()))
            }
        }
        children.push(text_to_node("\n", parent.clone()));
    }
    children
}

fn text_to_node(text: &str, parent: Handle) -> Handle {
    Rc::new(Node {
        parent: Cell::new(Some(Rc::downgrade(&parent))),
        children: RefCell::new(vec![]),
        data: NodeData::Text {
            contents: RefCell::new(text.to_owned().into()),
        },
    })
}

fn style_to_node(style: Style, text: &str, parent: Handle) -> Handle {
    let parent = Cell::new(Some(Rc::downgrade(&parent)));
    let style = Attribute {
        name: QualName {
            prefix: None,
            ns: ns!(),
            local: LocalName::from("style"),
        },
        value: style_to_attr(style).into(),
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
    let res = Rc::new(Node {
        parent,
        data,
        children: RefCell::new(Vec::new()),
    });
    res.children
        .borrow_mut()
        .push(text_to_node(text, res.clone()));
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
        write!(s, "#{:02x}{:02x}{:02x}{:02x}", c.r, c.g, c.b, c.a).unwrap();
    } else {
        write!(s, "#{:02x}{:02x}{:02x}", c.r, c.g, c.b).unwrap();
    }
}
