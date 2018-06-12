#[macro_use]
extern crate askama;
extern crate clap;
extern crate chrono;
extern crate html5ever_ext;
extern crate inflector;
extern crate pulldown_cmark;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate toml;
mod templates;
use templates::{IndexTemplate, AboutTemplate, NotFoundTemplate, BlogTemplate};

use askama::Template;
use clap::{App, Arg, SubCommand};
use chrono::{DateTime, Utc};
use html5ever_ext::RcDom;
use html5ever_ext::RcDomExt;
use html5ever_ext::Minify;
use pulldown_cmark::html::push_html;
use pulldown_cmark::{Parser, Options, OPTION_ENABLE_TABLES, OPTION_ENABLE_FOOTNOTES};

use std::collections::BTreeMap;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::fmt::Display;

/* TODO: Add build and serve functions, () -> Result<(), OpaqueError>, and call them in the
 *       subcomand matches
 *       Build will just create the html files directly ✓
 *       Minify the html files ✓
 *       Serve will spin up a simple web server, and define the routes based off of the blogs
 *       directly. Then, just update the blog everytime a file changes.
 * TODO: Add a template html/css wrapper
 * TODO: Add a deploy subcommand that takes care of the scp step
 */

#[derive(Debug)]
struct OpaqueError {
    msg: String,
}

impl OpaqueError {
    fn new<S: Into<String>>(msg: S) -> OpaqueError {
        OpaqueError { msg: msg.into() }
    }
}

impl Display for OpaqueError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

macro_rules! opaque_error {
    ($error:ty) => {
        impl From<$error> for OpaqueError {
            fn from(error: $error) -> Self {
                OpaqueError::new(format!("{}", error))
            }
        }
    }
}

opaque_error!(io::Error);
opaque_error!(clap::Error);
opaque_error!(toml::de::Error);
opaque_error!(toml::ser::Error);
opaque_error!(askama::Error);
opaque_error!(html5ever_ext::HtmlError);

fn snake_to_title(snake: &str) -> String {
    inflector::cases::sentencecase::to_sentence_case(snake)
}

#[derive(Debug)]
pub struct Blog {
    created: DateTime<Utc>,
    modified: DateTime<Utc>,
    title: String,
    link_text: String,
    article_html: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct Config {
    timings: BTreeMap<String, DateTime<Utc>>,
}

impl Config {
    fn from_file<P: AsRef<Path>>(config_file: P) -> Result<Self, OpaqueError> {
        let mut buffer = vec![];
        let mut r = BufReader::new(File::open(config_file)?);
        r.read_to_end(&mut buffer)?;
        Ok(toml::from_slice(&buffer[..]).unwrap_or(Config { timings: BTreeMap::new() }))
    }

    fn update_timings(&mut self, blogs: &Vec<Blog>) {
        let mut persist_timings = BTreeMap::new();
        for blog in blogs {
            persist_timings.insert(blog.link_text.clone(), blog.created.clone());
        };
        self.timings = persist_timings;
    }

    fn to_file<P: AsRef<Path>>(&self, config_file: P) -> Result<(), OpaqueError> {
        let metadata_content = toml::ser::to_vec(self)?;
        BufWriter::new(File::create(config_file)?)
            .write_all(&metadata_content[..]).map_err(|e| e.into())
    }
}

fn read_blogs(config: &Config, static_dir: &str) -> Result<Vec<Blog>, OpaqueError> {
    let mut blogs = vec![];
    let mut buf = String::new();
    let opts = {
        let mut opts = Options::empty();
        opts.insert(OPTION_ENABLE_FOOTNOTES);
        opts.insert(OPTION_ENABLE_TABLES);
        opts
    };

    for entry in fs::read_dir(static_dir)? {
        buf.clear();
        let entry = entry?;
        let path = entry.path();
        let extension = path.extension().and_then(std::ffi::OsStr::to_str);
        let link_text = path.file_name()
            .ok_or(OpaqueError::new("Path ending in ...!"))?
            .to_str()
            .ok_or(OpaqueError::new(format!("Invalid filename: {:?}", &path)))?
            .trim_right_matches(".md")
            .to_owned();

        if !path.is_file() || extension != Some("md") {
            continue;
        }

        let title = snake_to_title(&link_text);
        let metadata = entry.metadata()?;
        let modified: DateTime<Utc> = metadata.modified()?.into();
        // Get true created time, otherwise see if we have a creation time,
        // and lastly just use the modified time.
        let created:  DateTime<Utc> = metadata.created()
            .ok()
            .map(|st| st.into())
            .or_else(|| config.timings.get(&link_text).map(|dt| dt.clone()))
            .unwrap_or_else(|| modified.clone());
        let mut markdown_reader = BufReader::new(File::open(&path)?);
        markdown_reader.read_to_string(&mut buf)?;
        let markdown_parser = Parser::new_ext(&buf, opts);
        let mut article_html = String::new();
        push_html(&mut article_html, markdown_parser);
        blogs.push(Blog { title, link_text, article_html, modified, created });
    }
    Ok(blogs)
}

fn build_blogs<P: AsRef<Path>>(build_dir: P, blogs: Vec<Blog>)
    -> Result<(), OpaqueError>
{
    let mut path_buf = PathBuf::new();
    path_buf.push(build_dir);

    path_buf.push("index.html");
    write_template(&path_buf, IndexTemplate::new(&blogs[..]))?;
    path_buf.set_file_name("about.html");
    write_template(&path_buf, AboutTemplate::new())?;
    path_buf.set_file_name("404.html");
    write_template(&path_buf, NotFoundTemplate::new())?;

    for blog in blogs {
        path_buf.set_file_name(&blog.link_text);
        path_buf.set_extension("html");
        write_template(&path_buf, BlogTemplate::new(&blog))?;
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
    let arg_metadata = "METADATA_FILE";
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

    let metadata_file = matches.value_of(arg_metadata)
        .map(|s| Path::new(s).to_path_buf())
        .unwrap_or(Path::new(static_dir).join(".meta.toml"));

    let mut config = Config::from_file(&metadata_file).unwrap_or(Config::default());
    let blogs = read_blogs(&config, static_dir)?;

    config.update_timings(&blogs);
    config.to_file(&metadata_file)?;

    println!("{:?}", blogs);
    match matches.subcommand() {
        ("build", Some(build_matches)) => {
            let build_dir = build_matches.value_of(arg_build).unwrap_or("build");
            fs::create_dir_all(build_dir)?;
            build_blogs(build_dir, blogs)?;
            println!("Build dir: {}", build_dir);
            Ok(())
        },
        ("serve", Some(serve_matches)) => {
            let listen = serve_matches.value_of(arg_listen).unwrap_or("127.0.0.1:5867");
            println!("Listening on: {}", listen);
            Ok(())
        },
        _ => unreachable!()
    }
}
