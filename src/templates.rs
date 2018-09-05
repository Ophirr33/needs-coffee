use askama::Template;

#[derive(Debug, PartialEq)]
pub enum LinkType {
    Style,
    Script
}

#[derive(Debug)]
pub struct Link {
    name: String,
    link_type: LinkType
}

impl Link {
    fn new<S: ToString>(name: S, link_type: LinkType) -> Self {
        Link { name: name.to_string(), link_type }
    }
}

#[derive(Debug)]
pub struct Meta {
    name: String,
    content: String
}

#[derive(Debug, Template)]
#[template(path = "base.html")]
pub struct BaseTemplate {
    title: String,
    subtitle: String,
    browser_title: String,
    links: Vec<Link>,
    metas: Vec<Meta>,
    description: String,
}

impl BaseTemplate {
    fn new<S1: ToString, S2: ToString>(
        title: S1,
        subtitle: S1,
        browser_title: S1,
        description: S2,
        mut links: Vec<Link>,
        metas: Vec<Meta>) -> Self  {
        let base_css = Link::new("/styles.css", LinkType::Style);
        links.push(base_css);
        BaseTemplate {
            title: title.to_string(),
            subtitle: subtitle.to_string(),
            browser_title: browser_title.to_string(),
            description: description.to_string(),
            links,
            metas
        }
    }

}

#[derive(Debug)]
pub struct Blog {
    link: String,
    title: String,
    created: String,
}

impl Blog {
    pub fn new(link: String, title: String, created: String) -> Self {
        Blog { link, title, created }
    }
}

#[derive(Debug, Template)]
#[template(path = "index.html")]
pub struct IndexTemplate<'a> {
    _parent: BaseTemplate,
    blogs: &'a[Blog],
}

impl<'a> IndexTemplate<'a> {
    pub fn new(blogs: &'a[Blog]) -> Self {
        let description = "Ty Coghlan's personal website and coffee-fueled blog.";
        let date_script = Link::new("/date_script.js", LinkType::Script);
        let base = BaseTemplate::new(
            "TY COGHLAN",
            "Software Developer, Coffee Drinker",
            "Ty Coghlan | Ty Needs Coffee",
            description,
            vec![date_script],
            vec![]);
        IndexTemplate {
            _parent: base,
            blogs: blogs,
        }
    }
}

#[derive(Debug, Template)]
#[template(path = "blog.html", escape = "none")]
pub struct BlogTemplate<'a> {
    _parent: BaseTemplate,
    blog_html: &'a str,
}

impl<'a> BlogTemplate<'a> {
    pub fn new(blog_html: &'a str, blog: Blog) -> Self {
        let description = "Will make this an actual description eventually";
        let mut blog_browser_title = blog.title.clone();
        let suffix = " | Ty Needs Coffee";
        if blog_browser_title.len() < 70 - suffix.len() {
            blog_browser_title.push_str(suffix);
        }
        let base = BaseTemplate::new(
            blog.title.to_uppercase(),
            "By Ty Coghlan".to_owned(),
            blog_browser_title,
            description,
            vec![],
            vec![]);
        BlogTemplate {
            _parent: base,
            blog_html,
        }
    }
}

#[derive(Debug)]
pub struct LinkLabel {
    preview_link: String,
    image_link: String,
    label: String,
}

impl LinkLabel {
    pub fn new(preview_link: String, image_link: String, label: String) -> Self {
        LinkLabel { preview_link, image_link, label }
    }
}

#[derive(Debug, Template)]
#[template(path = "gallery.html")]
pub struct GalleryTemplate<'a> {
    _parent: BaseTemplate,
    label_links: &'a[LinkLabel],
}


impl<'a> GalleryTemplate<'a> {
    pub fn new(label_links: &'a[LinkLabel]) -> Self {
        let description = "Just my amateur photos";
        let base = BaseTemplate::new(
            "TY COGHLAN",
            "Occasional Photographer",
            "Gallery | Ty Needs Coffee", description, vec![], vec![]);
        GalleryTemplate {
            _parent: base,
            label_links,
        }
    }
}

#[derive(Debug, Template)]
#[template(path = "about.html")]
pub struct AboutTemplate {
    _parent: BaseTemplate
}

impl AboutTemplate {
    pub fn new() -> Self {
        let description = "Ty's bio, relevant links, and coffee preferences.";
        let base = BaseTemplate::new(
            "TY COGHLAN",
            "(No, it's not short for Tyler)",
            "About | Ty Needs Coffee",
            description,
            vec![],
            vec![]);
        AboutTemplate {
            _parent: base,
        }
    }
}

#[derive(Debug, Template)]
#[template(path = "404.html")]
pub struct NotFoundTemplate {
    _parent: BaseTemplate
}

impl NotFoundTemplate {
    pub fn new() -> Self {
        let description = "404 Page Not Found";
        let base = BaseTemplate::new(
            "404",
            "Page Not Found",
            "404 | Ty Needs Coffee",
            description,
            vec![],
            vec![]);
        NotFoundTemplate {
            _parent: base,
        }
    }
}


