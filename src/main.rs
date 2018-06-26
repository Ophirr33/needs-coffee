#[macro_use]
extern crate askama;
extern crate clap;
extern crate chrono;
extern crate html5ever_ext;
extern crate image;
extern crate inflector;
#[macro_use]
extern crate log;
extern crate simple_logger;
extern crate pulldown_cmark;
extern crate sass_rs;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate toml;
mod templates;
mod errors;
mod util;
use errors::*;

use templates::{IndexTemplate, AboutTemplate, NotFoundTemplate, BlogTemplate, GalleryTemplate};

use askama::Template;
use clap::{App, Arg, SubCommand};
use chrono::{DateTime, Utc};
use html5ever_ext::{RcDom, RcDomExt, Minify};
use pulldown_cmark::html::push_html;
use pulldown_cmark::{Parser, Options, OPTION_ENABLE_TABLES, OPTION_ENABLE_FOOTNOTES};
use sass_rs::{OutputStyle, compile_string};

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::ffi::OsString;

/* TODO: Add build and serve functions, () -> Result<(), OpaqueError>, and call them in the
 *       subcomand matches
 *       Build will just create the html files directly ✓
 *       Minify the html files ✓
 *       Compile and output sass styles file ✓
 *       Cleanly refactor
 *       Resize image files to both blog width, preview width, and gallery width ✓
 *       Don't reprocess images if files are already there ✓
 *       Serve will spin up a simple web server, and define the routes based off of the blogs
 *       directly. Then, just update the blog everytime a file changes.
 * TODO: Add a deploy subcommand that takes care of the scp step
 */

pub struct Photo {
    created: DateTime<Utc>,
    modified: DateTime<Utc>,
    name: String,
    image: Vec<u8>,
}

impl Photo {
    // creates a thumbnail that is 640 x 360
    fn create_thumbnail(name: String,
                        image: Vec<u8>,
                        modified: DateTime<Utc>,
                        created: DateTime<Utc>) -> Result<Self, OpaqueError>
    {
        Ok(Photo{ name, modified, created, image })
    }

    fn preview_link(&self) -> String {
        format!("thumbnails/{}.jpg", self.name)
    }
}

impl std::fmt::Debug for Photo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Photo({}, {}, {})", self.name, self.created, self.modified)
    }
}

#[derive(Debug)]
pub struct Blog {
    created: DateTime<Utc>,
    modified: DateTime<Utc>,
    name: String,
    article_html: String,
}

impl Blog {
    fn title(&self) -> String  {
        inflector::cases::sentencecase::to_sentence_case(&self.name)
    }

    fn blog_link(&self) -> String {
        format!("blog/{}.html", self.name)
    }

}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
struct Config {
    timings: BTreeMap<String, DateTime<Utc>>,
}

impl Config {
    fn from_file<P: AsRef<Path>>(config_file: P) -> Result<Self, OpaqueError> {
        Ok(toml::from_slice(util::read_file(config_file)?.as_bytes())?)
    }

    fn update_timings(&self, blogs: &[Blog], photos: &[Photo]) -> Config {
        let mut timings = BTreeMap::new();
        for blog in blogs {
            timings.insert(blog.name.clone(), blog.created.clone());
        };
        for photo in photos {
            timings.insert(photo.name.clone(), photo.created.clone());
        }
        Config { timings }
    }

    fn to_file<P: AsRef<Path>>(&self, config_file: P) -> Result<(), OpaqueError> {
        util::write_file(config_file, toml::ser::to_vec(self)?)
    }
}

fn read_blogs_photos(config: &Config, static_dir: &str)
    -> Result<(Vec<Blog>, Vec<Photo>), OpaqueError>
{
    let mut blogs = vec![];
    let mut photos = vec![];
    let opts = {
        let mut opts = Options::empty();
        opts.insert(OPTION_ENABLE_FOOTNOTES);
        opts.insert(OPTION_ENABLE_TABLES);
        opts
    };

    for entry in fs::read_dir(static_dir)? {
        let entry = entry?;
        let path = entry.path();
        let extension = path.extension().and_then(std::ffi::OsStr::to_str);
        let name = path.file_name()
            .ok_or(OpaqueError::new("Path ending in ...!"))?
            .to_str()
            .ok_or(OpaqueError::new(format!("Invalid filename: {:?}", &path)))?
            .trim_right_matches(".md")
            .trim_right_matches(".jpg")
            .to_owned();

        if !path.is_file() {
            continue;
        }

        let metadata = entry.metadata()?;
        let modified: DateTime<Utc> = metadata.modified()?.into();
        // Get true created time, otherwise see if we have a creation time,
        // and lastly just use the modified time.
        let created:  DateTime<Utc> = metadata.created()
            .ok()
            .map(|st| st.into())
            .or_else(|| config.timings.get(&name).map(|dt| dt.clone()))
            .unwrap_or_else(|| modified.clone());

        match extension {
            Some("md") => {
                let mut buf = util::read_file(&path)?;
                let markdown_parser = Parser::new_ext(&buf, opts);
                let mut article_html = String::new();
                push_html(&mut article_html, markdown_parser);
                blogs.push(Blog { name, article_html, modified, created });
            },
            Some("jpg") => {
                let mut buf = util::read_file_raw(&path)?;
                photos.push(Photo::create_thumbnail(name, buf, modified, created)?);
            },
            _ => continue
        }
    }
    Ok((blogs, photos))
}

fn write_style<P1: AsRef<Path>, P2: AsRef<Path>>(build_dir: P1, style_file: P2)
    -> Result<(), OpaqueError>
{
    let style_path: &Path = style_file.as_ref();
    let build_path: &Path = build_dir.as_ref();
    let sass = {
        let mut buf = util::read_file(style_path)?;
        let mut options = sass_rs::Options::default();
        options.output_style = OutputStyle::Compressed;
        compile_string(&buf, options).map_err(OpaqueError::new)?
    };
    let default = OsString::from("styles.css");
    let style_name = style_path.file_name().unwrap_or(&default);
    let out_file = build_path.join(style_name).with_extension("css");
    util::write_file(out_file, sass)?;
    Ok(())
}

fn write_html<P: AsRef<Path>>(build_dir: P, blogs: &[Blog], photos: &[Photo])
    -> Result<(), OpaqueError>
{
    let mut path_buf = build_dir.as_ref().join("index.html");
    write_template(&path_buf, IndexTemplate::new(&blogs[..]))?;
    path_buf.set_file_name("about.html");
    write_template(&path_buf, AboutTemplate::new())?;
    path_buf.set_file_name("404.html");
    write_template(&path_buf, NotFoundTemplate::new())?;
    path_buf.set_file_name("gallery.html");
    write_template(&path_buf, GalleryTemplate::new(photos))?;
    path_buf.set_file_name("blog");
    path_buf.push("temp_name");

    for blog in blogs {
        path_buf.set_file_name(&blog.name);
        path_buf.set_extension("html");
        write_template(&path_buf, BlogTemplate::new(&blog))?;
    }
    Ok(())
}

fn write_images<P: AsRef<Path>>(build_dir: P, photos: &[Photo])
    -> Result<(), OpaqueError>
{
    let mut images_path = build_dir.as_ref().join("images").join("_");
    let mut thumbnails_path = build_dir.as_ref().join("thumbnails").join("_");
    for photo in photos {
        images_path.set_file_name(&photo.name);
        images_path.set_extension("jpg");
        thumbnails_path.set_file_name(&photo.name);
        thumbnails_path.set_extension("jpg");
        if thumbnails_path.exists() && images_path.exists() {
            continue
        }
        use image::*;
        let image = load_from_memory_with_format(&photo.image, ImageFormat::JPEG)?;
        if !thumbnails_path.exists() {
            let thumbnail = image.resize(640, 360, image::FilterType::Triangle);
            thumbnail.save(&thumbnails_path)?;
        }
        if !images_path.exists() {
            let fullsize = image.resize(2560, 1440, image::FilterType::Triangle);
            fullsize.save(&images_path)?;
        }
    }
    Ok(())
}

fn write_template<T: Template>(path: &Path, temp: T)
    -> Result<(), OpaqueError>
{
    let rendered = temp.render()?;
    let dom = RcDom::from_bytes(rendered.as_bytes());
    dom.minify_to_file_path(false, path)
        .map_err(|e|e.into())
}

fn main() -> Result<(), OpaqueError> {
    askama::rerun_if_templates_changed();
    simple_logger::init()?;
    let arg_metadata = "METADATA_FILE";
    let arg_style    = "SASS_FILE";
    let arg_static   = "STATIC_DIR";
    let arg_build    = "BUILD_DIR";
    let arg_listen   = "LISTEN_ADDR";
    let mut app =
        App::new("static-site-generator")
            .version("1.0")
            .author("Ty Coghlan <coghlan.ty@gmail.com>")
            .about("Incredibly simple site generator")
            .arg(Arg::with_name(arg_static)
                     .short("s")
                     .long("static")
                     .help("The static site directory")
                     .takes_value(true)
                     .global(true))
            .arg(Arg::with_name(arg_metadata)
                     .short("m")
                     .long("metadata")
                     .help("The location of the metadata file (stores time info)")
                     .takes_value(true)
                     .global(true))
            .arg(Arg::with_name(arg_style)
                     .long("style")
                     .help("The location of the sass style file")
                     .takes_value(true)
                     .global(true))
            .subcommand(SubCommand::with_name("build")
                            .about("Builds published site files")
                            .arg(Arg::with_name(arg_build)
                                     .index(1)
                                     .help("The output directory")
                                     .takes_value(true)))
            .subcommand(SubCommand::with_name("serve")
                            .about("Just serves a hot reload server")
                            .arg(Arg::with_name(arg_listen)
                                     .index(1)
                                     .help("The address and port to listen on")
                                     .takes_value(true)));
    let matches = app.clone().get_matches();

    match matches.subcommand() {
        ("build", Some(_build_matches)) => { },
        ("serve", Some(_serve_matches)) => { },
        _ => {
            app.print_help()?;
            println!();
            return Ok(());
        },
    };

    let static_dir = matches.value_of(arg_static).unwrap_or("static");
    fs::create_dir_all(static_dir)?;

    let style_file    = matches.value_of(arg_style)
        .map(|s| Path::new(s).to_path_buf())
        .unwrap_or(Path::new(static_dir).join("styles.sass"));

    let metadata_file = matches.value_of(arg_metadata)
        .map(|s| Path::new(s).to_path_buf())
        .unwrap_or(Path::new(static_dir).join(".meta.toml"));

    let config = Config::from_file(&metadata_file).unwrap_or(Config::default());
    let (blogs, photos) = read_blogs_photos(&config, static_dir)?;
    let updated_config = config.update_timings(&blogs, &photos);

    if config != updated_config {
        updated_config.to_file(&metadata_file)?;
    }

    match matches.subcommand() {
        ("build", Some(build_matches)) => {
            let build_dir = Path::new(build_matches.value_of(arg_build).unwrap_or("build"));
            build(&build_dir, &style_file, &blogs, &photos)
        },
        ("serve", Some(serve_matches)) => {
            let _listen = serve_matches.value_of(arg_listen).unwrap_or("127.0.0.1:5867");
            Ok(())
        },
        _ => unreachable!()
    }
}

fn build(build_dir: &Path, style_file: &Path, blogs: &[Blog], photos: &[Photo]) -> Result<(), OpaqueError> {
    fs::create_dir_all(&build_dir.join("blog"))?;
    fs::create_dir_all(&build_dir.join("images"))?;
    fs::create_dir_all(&build_dir.join("thumbnails"))?;
    write_style(&build_dir, style_file)?;
    write_html(&build_dir, &blogs, &photos)?;
    write_images(&build_dir, &photos)?;
    Ok(())
}
