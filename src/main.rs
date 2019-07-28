extern crate clap;
extern crate env_logger;
extern crate log;

use clap::{App, Arg, SubCommand};
use std::env;
use std::path::Path;

fn main() {
    // Parse command line
    let cli = App::new("doublegit")
        .bin_name("doublegit")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(Arg::with_name("verbose")
             .short("v")
             .help("Augment verbosity (print more details)")
             .multiple(true))
        .subcommand(SubCommand::with_name("update")
                    .about("Fetch a repository and update its history")
                    .arg(Arg::with_name("repository")
                         .help("Path to repository")
                         .required(true)
                         .takes_value(true)));
    #[cfg(feature = "web")]
    let cli = cli.subcommand(SubCommand::with_name("web")
                             .about("Open the repository history web page")
                             .arg(Arg::with_name("host")
                                  .short("-h")
                                  .help("Set the host to bind the server to")
                                  .takes_value(true)
                                  .default_value("127.0.0.1"))
                             .arg(Arg::with_name("port")
                                  .short("-p")
                                  .help("Set the port number for the server")
                                  .takes_value(true)
                                  .default_value("6617")));

    let mut cli = cli;
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

    match matches.subcommand_name() {
        Some("update") => {
            let s_matches = matches.subcommand_matches("update").unwrap();
            let repository = s_matches.value_of_os("repository").unwrap();
            let repository = Path::new(repository);
            match doublegit::update(repository) {
                Ok(()) => {},
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        }
        Some("web") => {
            let s_matches = matches.subcommand_matches("web").unwrap();
            let host = s_matches.value_of("host").unwrap();
            let port = match s_matches.value_of("port").unwrap().parse() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Invalid port number: {}", e);
                    std::process::exit(2);
                }
            };
            doublegit::web::serve(host, port);
        }
        _ => {
            cli.print_help().expect("Can't print help");
            std::process::exit(2);
        }
    }
}
