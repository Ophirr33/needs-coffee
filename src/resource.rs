use askama::Template;
use config::{Config, Timing};
use errors::{OResult, OpaqueError};
use inflector::cases::titlecase::to_title_case;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use templates::LinkLabel;
use util;

const CSS: &'static str = "css";
const HTML: &'static str = "html";
const ICO: &'static str = "ico";
const JPG: &'static str = "jpg";
const JS: &'static str = "js";
const MD: &'static str = "md";
const SASS: &'static str = "sass";
const BLOG_DIR: &'static str = "blog";
const IMAGE_DIR: &'static str = "image";
const THUMBNAIL_DIR: &'static str = "thumbnail";

use templates::{
    AboutTemplate, Blog, BlogTemplate, GalleryTemplate, IndexTemplate, NotFoundTemplate,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceType {
    Blog,
    Icon,
    Photo,
    Style,
    Script,
}

impl ResourceType {
    fn from_extension(ext: &str) -> OResult<Self> {
        match ext {
            MD => Ok(ResourceType::Blog),
            JPG => Ok(ResourceType::Photo),
            SASS => Ok(ResourceType::Style),
            JS => Ok(ResourceType::Script),
            ICO => Ok(ResourceType::Icon),
            _ => Err(OpaqueError::new(format!(
                "No resource type for extension {}",
                ext
            ))),
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
        Blog::new(
            format!("{}/{}.{}", BLOG_DIR, self.name, HTML),
            to_title_case(&self.name),
            self.timing.created.to_rfc3339(),
        )
    }

    fn path_exists(&self, build_dir: &Path) -> bool {
        match self.resource_type {
            ResourceType::Blog => build_dir
                .join(BLOG_DIR)
                .join(&self.name)
                .with_extension(HTML)
                .exists(),
            ResourceType::Script => build_dir.join(&self.name).with_extension(JS).exists(),
            ResourceType::Style => build_dir.join(&self.name).with_extension(CSS).exists(),
            ResourceType::Photo => {
                build_dir
                    .join(IMAGE_DIR)
                    .join(&self.name)
                    .with_extension(JPG)
                    .exists()
                    && build_dir
                        .join(THUMBNAIL_DIR)
                        .join(&self.name)
                        .with_extension(JPG)
                        .exists()
            }
            ResourceType::Icon => build_dir.join(&self.name).with_extension(ICO).exists(),
        }
    }

    fn write_resource(&self, build_dir: &Path) -> OResult<()> {
        match self.resource_type {
            ResourceType::Blog => self.write_blog(build_dir),
            ResourceType::Script => self.copy_resource(build_dir, JS), //TODO: minify
            ResourceType::Style => self.write_style(build_dir),
            ResourceType::Photo => self.write_photo(build_dir),
            ResourceType::Icon => self.copy_resource(build_dir, ICO),
        }
    }

    fn copy_resource(&self, build_dir: &Path, ext: &str) -> OResult<()> {
        let out_file = &build_dir.join(&self.name).with_extension(ext);
        info!("Copying resource to {:?}", out_file);
        util::write_file(out_file, util::read_bytes(&self.path)?)
    }

    fn write_style(&self, build_dir: &Path) -> OResult<()> {
        use super::sass_rs::*;
        let mut options = Options::default();
        options.output_style = OutputStyle::Compressed;
        info!("Reading style file from {:?}", &self.path);
        let sass = compile_file(&self.path, options).map_err(OpaqueError::new)?;
        let css_file = build_dir.join(&self.name).with_extension(CSS);
        info!("Building style file {} to {:?}", self.name, css_file);
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
        info!("Reading blog from {:?}", self.path);
        let buf = util::read_file(&self.path)?;
        let markdown_parser = Parser::new_ext(&buf, opts);
        let mut html_buf = String::new();
        html::push_html(&mut html_buf, markdown_parser);
        let blog = BlogTemplate::new(&html_buf, self.as_blog());
        let blog_file = build_dir
            .join(BLOG_DIR)
            .join(&self.name)
            .with_extension(HTML);
        info!("Writing blog file {} to {:?}", self.name, blog_file);
        util::write_minified_html(blog_file, blog.render()?)?;
        Ok(())
    }

    fn write_photo(&self, build_dir: &Path) -> OResult<()> {
        use image::*;
        info!("Reading photo from {:?}", self.path);
        let image = load(BufReader::new(File::open(&self.path)?), ImageFormat::JPEG)?;
        let thumbnail_path = build_dir
            .join(THUMBNAIL_DIR)
            .join(&self.name)
            .with_extension(JPG);
        let thumbnail = image.resize(640, 360, FilterType::Triangle);
        info!("Building photo thumbnail to {:?}", thumbnail_path);
        thumbnail.save(&thumbnail_path)?;
        let fullsize_path = build_dir
            .join(IMAGE_DIR)
            .join(&self.name)
            .with_extension(JPG);
        let fullsize = image.resize(1280, 720, FilterType::Triangle);
        info!("Building fullsize photo to {:?}", fullsize_path);
        fullsize.save(&fullsize_path)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SiteResources {
    resources: Vec<SiteResource>,
}

impl SiteResources {
    pub fn read_resources(static_dir: &Path, config: &Config) -> OResult<Self> {
        let mut resources = vec![];

        info!("Reading resources from static directory {:?}", static_dir);
        for entry in fs::read_dir(static_dir)? {
            let entry = entry?;
            let path = entry.path();
            let extension = path
                .extension()
                .and_then(OsStr::to_str)
                .ok_or(OpaqueError::new("No file extension!"))?
                .to_owned();

            let resource_type = match ResourceType::from_extension(&extension) {
                Err(_) => {
                    info!("Skipping file due to unknown extension: {:?}", &path);
                    continue;
                }
                Ok(t) => t,
            };

            let name = path
                .file_name()
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
            let changed = prev
                .map(|prev_timing| prev_timing != &timing)
                .unwrap_or(true);

            resources.push(SiteResource {
                timing,
                changed,
                path,
                name,
                resource_type,
            })
        }
        // sort newest to oldest
        resources.sort_unstable_by_key(|r| r.timing.created);
        resources.reverse();
        Ok(SiteResources { resources })
    }

    pub fn timings(&self) -> BTreeMap<String, Timing> {
        self.resources
            .iter()
            .map(|r| (r.name.clone(), r.timing.clone()))
            .collect()
    }

    pub fn build_all(&self, build_dir: &Path, ignore_changed: bool) -> OResult<()> {
        Self::create_dir_if_not_exists(build_dir)?;
        Self::create_dir_if_not_exists(&build_dir.join(BLOG_DIR))?;
        Self::create_dir_if_not_exists(&build_dir.join(IMAGE_DIR))?;
        Self::create_dir_if_not_exists(&build_dir.join(THUMBNAIL_DIR))?;
        info!("Writing resources into build directory {:?}", build_dir);
        self.write_gallery(build_dir)?;
        self.write_index(build_dir)?;
        self.write_static_templates(build_dir)?;
        self.write_resources(build_dir, ignore_changed)?;
        info!("Done");
        Ok(())
    }

    fn create_dir_if_not_exists(dir: &Path) -> OResult<()> {
        if !dir.exists() {
            info!("Creating {:?}", dir);
            fs::create_dir_all(dir)?;
        };
        Ok(())
    }

    fn write_resources(&self, build_dir: &Path, ignore_changed: bool) -> OResult<()> {
        self.resources
            .par_iter()
            .filter(|r| r.changed || ignore_changed || !r.path_exists(build_dir))
            .map(|r| r.write_resource(build_dir))
            .collect()
    }

    fn write_gallery(&self, build_dir: &Path) -> OResult<()> {
        let all_photos = self
            .resources
            .iter()
            .filter(|r| r.resource_type == ResourceType::Photo)
            .map(|r| {
                LinkLabel::new(
                    format!("{}/{}.{}", THUMBNAIL_DIR, r.name, JPG),
                    format!("{}/{}.{}", IMAGE_DIR, r.name, JPG),
                    r.name.clone(),
                )
            }).collect::<Vec<_>>();
        let gallery = GalleryTemplate::new(&all_photos[..]);
        let gallery_path = build_dir.join("gallery.html");
        info!("Writing gallery file to {:?}", gallery_path);
        util::write_minified_html(gallery_path, gallery.render()?)?;
        Ok(())
    }

    fn write_index(&self, build_dir: &Path) -> OResult<()> {
        let all_blogs = self
            .resources
            .iter()
            .filter(|r| r.resource_type == ResourceType::Blog)
            .map(|r| r.as_blog())
            .collect::<Vec<_>>();
        let index = IndexTemplate::new(&all_blogs[..]);
        let index_path = build_dir.join("index.html");
        info!("Writing index file to {:?}", index_path);
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
            info!("Writing static template to {:?}", template_path);
            util::write_minified_html(template_path, template.render()?)?;
        }
        Ok(())
    }
}
