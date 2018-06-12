use askama::Template;
use super::*;

#[derive(Debug, Template)]
#[template(path = "base.html")]
pub struct BaseTemplate<'a> {
    title: &'a str,
    description: &'a str,
}

impl<'a> BaseTemplate<'a> {
    fn new(title: &'a str, description: &'a str) -> Self  {
        BaseTemplate { title, description }
    }
}

#[derive(Debug, Template)]
#[template(path = "index.html")]
pub struct IndexTemplate<'a> {
    _parent: BaseTemplate<'a>,
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
    _parent: BaseTemplate<'a>,
    blog: &'a Blog,
}

impl<'a> BlogTemplate<'a> {
    pub fn new(blog: &'a Blog) -> Self {
        let description = "Will make this an actual descriptioin eventually";
        BlogTemplate {
            _parent: BaseTemplate::new(&blog.title, description),
            blog,
        }
    }
}

#[derive(Debug, Template)]
#[template(path = "about.html")]
pub struct AboutTemplate<'a> {
    _parent: BaseTemplate<'a>
}

impl<'a> AboutTemplate<'a> {
    pub fn new() -> Self {
        let description = "Ty's bio, relevant links, and coffee preferences.";
        AboutTemplate {
            _parent: BaseTemplate::new("About Ty", description),
        }
    }
}

#[derive(Debug, Template)]
#[template(path = "404.html")]
pub struct NotFoundTemplate<'a> {
    _parent: BaseTemplate<'a>
}

impl<'a> NotFoundTemplate<'a> {
    pub fn new() -> Self {
        let description = "404 not found";
        NotFoundTemplate {
            _parent: BaseTemplate::new("404 not found", description),
        }
    }
}
