extern crate clap;
extern crate chrono;
extern crate inflector;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate toml;

use clap::{App, Arg, SubCommand};
use chrono::{DateTime, Utc};

use std::collections::BTreeMap;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::fs::{self, File};
use std::path::Path;
use std::fmt::Display;

/* TODO: Add build and serve functions, () -> Result<(), OpaqueError>, and call them in the subcomand matches
 *       Build will just create the html files directly
 *       Serve will spin up a simple web server, and define the routes based off of the blogs
 *       directly. Then, just update the blog evertime a file changes.
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

fn snake_to_title(snake: &str) -> String {
    inflector::cases::sentencecase::to_sentence_case(snake)
}

#[derive(Debug)]
struct Blog<T: Read> {
    created: DateTime<Utc>,
    modified: DateTime<Utc>,
    title: String,
    link_text: String,
    markdown: T,
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

    fn update_timings<T: Read>(&mut self, blogs: &Vec<Blog<T>>) {
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


fn read_blogs(config: &Config, static_dir: &str) -> Result<Vec<Blog<BufReader<File>>>, OpaqueError> {
    let mut blogs = vec![];

    for entry in fs::read_dir(static_dir)? {
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
        let markdown = BufReader::new(File::open(&path)?);
        let blog = Blog { title, link_text, markdown, modified, created };
        blogs.push(blog);
    }
    Ok(blogs)
}

fn main() -> Result<(), OpaqueError> {
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
