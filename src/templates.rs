use askama::Template;

#[derive(Debug, Template)]
#[template(path = "base.html")]
pub struct BaseTemplate {
    title: String,
    description: String,
}

impl BaseTemplate {
    fn new<S1: ToString, S2: ToString>(title: S1, description: S2) -> Self  {
        BaseTemplate { title: title.to_string(), description: description.to_string() }
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
        IndexTemplate {
            _parent: BaseTemplate::new("Index", description),
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
        BlogTemplate {
            _parent: BaseTemplate::new(blog.title, description),
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
        GalleryTemplate {
            _parent: BaseTemplate::new("Ty's Photography", description),
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
        AboutTemplate {
            _parent: BaseTemplate::new("About Ty", description),
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
        let description = "404 not found";
        NotFoundTemplate {
            _parent: BaseTemplate::new("404 not found", description),
        }
    }
}


