extern crate chrono;
extern crate erased_serde;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate regex;
extern crate rusqlite;
#[cfg(feature = "web")] #[macro_use] extern crate serde;
#[cfg(feature = "web")] #[macro_use] extern crate serde_json;
#[cfg(test)] extern crate tempfile;

pub mod apis;
mod git;
#[cfg(feature = "web")] pub mod web;

use std::fmt;
use std::path::Path;
use std::time::SystemTime;

/// Error type for this crate
#[derive(Debug)]
pub enum Error {
    /// An error with SQLite
    Sqlite(rusqlite::Error),
    /// An error calling Git
    Git(String),
    /// A general I/O error
    Io(std::io::Error),
    /// A configuration error
    Config(String),
}

impl Error {
    /// Utility to create a Git variant from a string
    fn git(msg: &str) -> Error {
        Error::Git(msg.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Sqlite(e) => write!(f, "SQLite error: {}", e),
            Error::Git(e) => write!(f, "Git error: {}", e),
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::Config(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Error {
        Error::Sqlite(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::Io(e)
    }
}

/// Update a project
pub fn update(project: &Path) -> Result<(), Error> {
    update_with_date(project, SystemTime::now())
}

/// Update a project given the current date
pub fn update_with_date<Date>(
    project: &Path,
    date: Date,
) -> Result<(), Error>
where
    Date: Into<chrono::DateTime<chrono::Utc>>,
{
    let date = date.into();

    // Update Git data
    git::update_with_date(project, date)?;

    // Update API data
    apis::update_with_date(project, date)?;

    Ok(())
}
