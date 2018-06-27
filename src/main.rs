#[macro_use]
extern crate askama;
extern crate clap;
extern crate chrono;
extern crate html5ever_ext;
extern crate image;
extern crate inflector;
#[macro_use]
extern crate log;
extern crate simplelog;
extern crate pulldown_cmark;
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

use clap::{App, Arg, SubCommand};
use config::*;
use errors::*;
use resource::SiteResources;
use std::path::{Path, PathBuf};

/* TODO: Add build and serve functions, () -> Result<(), OpaqueError>, and call them in the
 *       subcomand matches
 *       Build will just create the html files directly ✓
 *       Minify the html files ✓
 *       Compile and output sass styles file ✓
 *       Cleanly refactor
 *       Add caching to each resource
 *       Add a clean command that removes everything in build that isn't in static
 *       Resize image files to both blog width, preview width, and gallery width ✓
 *       Don't reprocess images if files are already there ✓
 *       Serve will spin up a simple web server, and define the routes based off of the blogs
 *       directly. Then, just update the blog everytime a file changes.
 * TODO: Add a deploy subcommand that takes care of the scp step
 */

fn main() -> Result<(), OpaqueError> {
    askama::rerun_if_templates_changed();
    simplelog::TermLogger::init(simplelog::LevelFilter::Debug,
                                simplelog::Config::default())?;
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

    let static_dir = Path::new(matches.value_of(arg_static).unwrap_or("static"));

    let metadata_file = matches.value_of(arg_metadata)
        .map(|s| PathBuf::from(s))
        .unwrap_or(static_dir.join(".meta.toml"));

    let config = Config::from_file(&metadata_file).unwrap_or(Config::default());
    let resources = SiteResources::read_resources(&static_dir, &config)?;
    let updated_config = Config::new(resources.timings());
    updated_config.to_file(&metadata_file)?;

    if config != updated_config {
        updated_config.to_file(&metadata_file)?;
    }

    match matches.subcommand() {
        ("build", Some(build_matches)) => {
            let build_dir = Path::new(build_matches.value_of(arg_build).unwrap_or("build"));
            resources.write_resources(build_dir)
        },
        ("serve", Some(serve_matches)) => {
            let _listen = serve_matches.value_of(arg_listen).unwrap_or("127.0.0.1:5867");
            Ok(())
        },
        _ => unreachable!()
    }
}
