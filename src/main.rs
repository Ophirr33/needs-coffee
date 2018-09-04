#[macro_use]
extern crate askama;
extern crate clap;
extern crate chrono;
extern crate html5ever_ext;
extern crate image;
extern crate inflector;
#[macro_use]
extern crate log;
extern crate notify;
extern crate simplelog;
extern crate pulldown_cmark;
extern crate rayon;
extern crate sass_rs;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate toml;
mod util;
mod errors;
mod config;
mod templates;
mod resource;
mod serve;

use clap::{App, Arg, SubCommand};
use config::*;
use errors::*;
use resource::SiteResources;
use serve::serve;
use std::path::{Path, PathBuf};
use std::fs;

/* TODO: Add build and serve functions, () -> Result<(), OpaqueError>, and call them in the
 *       subcomand matches
 *       Build will just create the html files directly ✓
 *       Minify the html files ✓
 *       Compile and output sass styles file ✓
 *       Cleanly refactor
 *       Add caching to each resource ✓
 *       Add a clean command that removes everything in build that isn't in static ✓
 *       Resize image files to both blog width, preview width, and gallery width ✓
 *       Don't reprocess images if files are already there ✓
 *       Add a watermark to images
 *       Serve will spin up a simple web server, and define the routes based off of the blogs
 *       directly. Then, just update the blog everytime a file changes.
 * TODO: Add a deploy subcommand that takes care of the scp step
 */

fn main() -> Result<(), OpaqueError> {
    simplelog::TermLogger::init(simplelog::LevelFilter::Info,
                                simplelog::Config::default())?;
    let arg_build    = "BUILD_DIR";
    let arg_cache    = "NO_CACHE";
    let arg_clean    = "CLEAN";
    let arg_listen   = "LISTEN_ADDR";
    let arg_metadata = "METADATA_FILE";
    let arg_static   = "STATIC_DIR";
    let mut app =
        App::new("static-site-generator")
            .version("1.0")
            .author("Ty Coghlan <coghlan.ty@gmail.com>")
            .about("Incredibly simple site generator")
            .arg(Arg::with_name(arg_static)
                     .long("static")
                     .help("The static site directory")
                     .takes_value(true)
                     .global(true))
            .arg(Arg::with_name(arg_metadata)
                     .long("metadata")
                     .help("The location of the metadata file (stores time info)")
                     .takes_value(true)
                     .global(true))
            .arg(Arg::with_name(arg_build)
                     .index(1)
                     .help("The output directory")
                     .takes_value(true))
            .subcommand(SubCommand::with_name("build")
                            .about("Builds published site files")
                            .arg(Arg::with_name(arg_clean)
                                     .long("clean")
                                     .help("removes the build directory")
                                     .takes_value(false))
                            .arg(Arg::with_name(arg_cache)
                                     .long("no-cache")
                                     .help("rebuilds all files regardless of timing")
                                     .takes_value(false)))
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

    let static_dir = Path::new(matches.value_of(arg_static).unwrap_or("static"));
    let build_dir = Path::new(matches.value_of(arg_build).unwrap_or("build"));

    let metadata_file = matches.value_of(arg_metadata)
        .map(|s| PathBuf::from(s))
        .unwrap_or(static_dir.join(".meta.toml"));

    let config = Config::from_file(&metadata_file).unwrap_or(Config::default());

    match matches.subcommand() {
        ("build", Some(build_matches)) => {
            let resources = SiteResources::read_resources(&static_dir, &config)?;
            if build_matches.is_present(arg_clean) {
                warn!("Cleaning build_dir {:?}", build_dir);
                fs::remove_dir_all(build_dir)?;
            }
            resources.build_all(build_dir, build_matches.is_present(arg_cache))?;
            let updated_config = Config::new(resources.timings());
            if config != updated_config {
                updated_config.to_file(&metadata_file)?;
            }
            Ok(())
        },
        ("serve", Some(_serve_matches)) => {
            serve(&config, &build_dir, &static_dir, &metadata_file)
        },
        _ => unreachable!()
    }
}
