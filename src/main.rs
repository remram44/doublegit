extern crate clap;
extern crate env_logger;
extern crate log;

use clap::{App, Arg};
use std::env;
use std::path::Path;

fn main() {
    // Parse command line
    let mut cli = App::new("doublegit")
        .bin_name("doublegit")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(Arg::with_name("verbose")
             .short("v")
             .help("Augment verbosity (print more details)")
             .multiple(true))
        .arg(Arg::with_name("repository")
             .help("Path to repository")
             .required(true)
             .takes_value(true));
    let matches = match cli.get_matches_from_safe_borrow(env::args_os()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(2);
        }
    };

    // Set up logging
    {
        let level = match matches.occurrences_of("verbose") {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        };
        let mut logger_builder = env_logger::builder();
        logger_builder.filter(None, level);
        if let Ok(val) = env::var("DOUBLEGIT_LOG") {
            logger_builder.parse_filters(&val);
        }
        if let Ok(val) = env::var("DOUBLEGIT_LOG_STYLE") {
            logger_builder.parse_write_style(&val);
        }
        logger_builder.init();
    }

    let repository = matches.value_of_os("repository").unwrap();
    let repository = Path::new(repository);
    match doublegit::update(repository) {
        Ok(()) => {},
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
