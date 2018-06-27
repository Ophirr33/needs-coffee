use askama::Template;
use config::{Config, Timing};
use errors::{OpaqueError, OResult};
use inflector::cases::sentencecase::to_sentence_case;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use util;

use templates::{
    AboutTemplate,
    Blog,
    BlogTemplate,
    GalleryTemplate,
    IndexTemplate,
    NotFoundTemplate
};

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceType {
    Blog,
    Photo,
    Style,
}

impl ResourceType {
    fn from_extension(ext: &str) -> OResult<Self> {
        match ext {
            "md" => Ok(ResourceType::Blog),
            "jpg" => Ok(ResourceType::Photo),
            "sass" => Ok(ResourceType::Style),
            _ => Err(OpaqueError::new(format!("No resource type for extension {}", ext))),
        }
    }
}

#[derive(Debug)]
pub struct SiteResource {
    timing: Timing,
    changed: bool,
    path: PathBuf,
    name: String,
    resource_type: ResourceType,
}

impl SiteResource {
    fn as_blog(&self) -> Blog {
        Blog::new(format!("blog/{}.html", self.name),
                  to_sentence_case(&self.name),
                  format!("{}", self.timing.created))
    }

    fn write_style(&self, build_dir: &Path) -> OResult<()> {
        use super::sass_rs::*;
        let mut options = Options::default();
        options.output_style = OutputStyle::Compressed;
        debug!("Reading style file from {:?}", &self.path);
        let sass = compile_file(&self.path, options).map_err(OpaqueError::new)?;
        let css_file = build_dir.join(&self.name).with_extension("css");
        debug!("Building style file {} to {:?}", self.name, css_file);
        util::write_file(css_file, sass)
    }

    fn write_blog(&self, build_dir: &Path) -> OResult<()> {
        use super::pulldown_cmark::*;
        let opts = {
            let mut opts = Options::empty();
            opts.insert(OPTION_ENABLE_FOOTNOTES);
            opts.insert(OPTION_ENABLE_TABLES);
            opts
        };
        debug!("Reading blog from {:?}", self.path);
        let buf = util::read_file(&self.path)?;
        let markdown_parser = Parser::new_ext(&buf, opts);
        let mut html_buf = String::new();
        html::push_html(&mut html_buf, markdown_parser);
        let blog = BlogTemplate::new(&html_buf, self.as_blog());
        let blog_file = build_dir.join("blog")
            .join(&self.name)
            .with_extension("html");
        debug!("Writing blog file {} to {:?}", self.name, blog_file);
        util::write_minified_html(blog_file, blog.render()?)?;
        Ok(())
    }

    fn write_photo(&self, build_dir: &Path) -> OResult<()> {
        use image::*;
        debug!("Reading photo from {:?}", self.path);
        let image = load(BufReader::new(File::open(&self.path)?), ImageFormat::JPEG)?;
        let thumbnail_path = build_dir.join("thumbnail")
            .with_file_name(&self.name)
            .with_extension("jpg");
        let thumbnail = image.resize(640, 360, FilterType::Triangle);
        debug!("Building photo thumbnail to {:?}", thumbnail_path);
        thumbnail.save(&thumbnail_path)?;
        let fullsize_path = build_dir.join("image")
            .with_file_name(&self.name)
            .with_extension("jpg");
        let fullsize = image.resize(2560, 1440, FilterType::Triangle);
        debug!("Building fullsize photo to {:?}", fullsize_path);
        fullsize.save(&fullsize_path)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SiteResources {
    resources: Vec<SiteResource>
}

impl SiteResources {
    pub fn read_resources(static_dir: &Path, config: &Config) -> OResult<Self> {
        let mut resources = vec![];

        debug!("Reading resources from static directory {:?}", static_dir);
        for entry in fs::read_dir(static_dir)? {
            let entry = entry?;
            let path = entry.path();
            let extension = path.extension()
                .and_then(OsStr::to_str)
                .ok_or(OpaqueError::new("No file extension!"))?.to_owned();

            if !["md", "jpg", "sass"].contains(&extension.as_ref()) {
                info!("Skipping file due to unknown extension: {:?}", &path);
                continue;
            }

            let name = path.file_name()
                .ok_or(OpaqueError::new("Path ending in ...!"))?
                .to_str()
                .ok_or(OpaqueError::new(format!("Invalid filename: {:?}", &path)))?
                .trim_right_matches(&format!(".{}", extension))
                .to_owned();

            if !path.is_file() {
                continue;
            }

            let metadata = entry.metadata()?;
            let prev = config.timings.get(&name);
            let timing = Timing::from_metadata_and_prev(&metadata, prev)?;
            let resource_type = ResourceType::from_extension(&extension)?;
            let changed = prev.map(|prev_timing| prev_timing != &timing).unwrap_or(true);

            resources.push(SiteResource { timing, changed, path, name, resource_type })
        }
        // sort newest to oldest
        resources.sort_unstable_by_key(|r| r.timing.created);
        resources.reverse();
        Ok(SiteResources { resources })
    }

    pub fn timings(&self) -> BTreeMap<String, Timing> {
        self.resources.iter().map(|r| (r.name.clone(), r.timing.clone())).collect()
    }

    pub fn write_resources(&self, build_dir: &Path) -> OResult<()> {
        debug!("Creating build, build/blog, build/image, and build/thumbnail");
        fs::create_dir_all(build_dir)?;
        fs::create_dir_all(build_dir.join("blog"))?;
        fs::create_dir_all(build_dir.join("image"))?;
        fs::create_dir_all(build_dir.join("thumbnail"))?;
        debug!("Writing resources into build directory {:?}", build_dir);
        self.write_styles(build_dir)?;
        self.write_blogs(build_dir)?;
        self.write_photos(build_dir)?;
        self.write_gallery(build_dir)?;
        self.write_index(build_dir)?;
        self.write_static_templates(build_dir)?;
        Ok(())
    }

    fn write_styles(&self, build_dir: &Path) -> OResult<()> {
        self.resources
            .iter()
            .filter(|r| r.changed && r.resource_type == ResourceType::Style)
            .map(|r| r.write_style(build_dir))
            .collect()
    }

    fn write_blogs(&self, build_dir: &Path) -> OResult<()> {
        self.resources
            .iter()
            .filter(|r| r.changed && r.resource_type == ResourceType::Blog)
            .map(|r| r.write_blog(build_dir))
            .collect()
    }

    fn write_photos(&self, build_dir: &Path) -> OResult<()> {
        self.resources
            .iter()
            .filter(|r| r.changed && r.resource_type == ResourceType::Photo)
            .map(|r| r.write_photo(build_dir))
            .collect()
    }

    fn write_gallery(&self, build_dir: &Path) -> OResult<()> {
        let all_photos = self.resources
            .iter()
            .filter(|r| r.resource_type == ResourceType::Photo)
            .map(|r| format!("thumbnail/{}.jpg", r.name))
            .collect::<Vec<_>>();
        let gallery = GalleryTemplate::new(&all_photos[..]);
        let gallery_path = build_dir.join("gallery.html");
        debug!("Writing gallery file to {:?}", gallery_path);
        util::write_minified_html(gallery_path, gallery.render()?)?;
        Ok(())
    }

    fn write_index(&self, build_dir: &Path) -> OResult<()> {
        let all_blogs = self.resources
            .iter()
            .filter(|r| r.resource_type == ResourceType::Blog)
            .map(|r| r.as_blog())
            .collect::<Vec<_>>();
        let index = IndexTemplate::new(&all_blogs[..]);
        let index_path = build_dir.join("index.html");
        debug!("Writing index file to {:?}", index_path);
        util::write_minified_html(index_path, index.render()?)?;
        Ok(())
    }

    fn write_static_templates(&self, build_dir: &Path) -> OResult<()> {
        let templates: Vec<(&'static str, Box<Template>)> = vec![
            ("404.html", Box::new(NotFoundTemplate::new())),
            ("about.html", Box::new(AboutTemplate::new())),
        ];
        for (file_name, template) in templates.into_iter() {
            let template_path = build_dir.join(file_name);
            debug!("Writing static template to {:?}", template_path);
            util::write_minified_html(template_path, template.render()?)?;
        }
        Ok(())
    }
}
