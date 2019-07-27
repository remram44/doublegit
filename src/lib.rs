extern crate chrono;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate regex;
extern crate rusqlite;

use rusqlite::Connection;
use rusqlite::types::ToSql;
use std::borrow::Cow;
use std::fmt;
use std::path::Path;
use std::time::SystemTime;

mod git;

#[derive(Debug)]
pub enum Error {
    Sqlite(rusqlite::Error),
    Git(String),
    Io(std::io::Error),
}

impl Error {
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

#[derive(Clone, Copy)]
enum Operation {
    FastForward,
    Forced,
    Pruned,
    Tag,
    New,
    Reject,
    Noop,
}

#[derive(PartialEq, Eq, Hash)]
pub struct Ref {
    remote: String,
    name: String,
    tag: bool,
}

impl Ref {
    fn parse_remote_ref(
        refname: &str,
        remote: &str,
    ) -> Result<Ref, Error> {
        let idx = refname.find('/')
            .ok_or(Error::git("Invalid remote ref"))?;
        let remote_part = &refname[0..idx];
        if remote_part != remote {
            return Err(Error::git("Remote ref has invalid remote"));
        }
        let name = &refname[idx + 1..];
        Ok(Ref { remote: remote.into(), name: name.into(), tag: false })
    }

    fn fullname(&self) -> Cow<String> {
        if self.tag {
            Cow::Borrowed(&self.name)
        } else {
            Cow::Owned(format!("{}/{}", self.remote, self.name))
        }
    }
}

pub fn update(repository: &Path) -> Result<(), Error> {
    update_with_date(repository, SystemTime::now())
}

pub fn update_with_date<Date>(
    repository: &Path,
    date: Date,
) -> Result<(), Error>
where
    Date: Into<chrono::DateTime<chrono::Utc>>,
{
    info!("Updating {:?}...", repository);

    // Open database
    let mut db = {
        let db_path = repository.join("gitarchive.sqlite3");
        let exists = db_path.exists();
        let db = Connection::open(db_path)?;
        if !exists {
            warn!("Database doesn't exist, creating tables...");
            db.execute(
                "
                CREATE TABLE refs(
                    remote TEXT NOT NULL,
                    name TEXT NOT NULL,
                    from_date DATETIME NOT NULL,
                    to_date DATETIME NULL,
                    sha TEXT NOT NULL,
                    tag BOOLEAN NOT NULL
                );
                ",
                rusqlite::NO_PARAMS,
            )?;
        }
        db
    };
    let tx = db.transaction()?;

    // Do fetch
    let out = git::fetch(repository)?;

    // Convert time to string
    let date = date.into().format("%Y-%m-%d %H:%M:%S").to_string();

    // Update database
    for ref_ in out.removed.iter().chain(out.changed.iter()) {
        tx.execute(
            "
            UPDATE refs SET to_date=?
            WHERE remote=? AND name=?
            ORDER BY from_date DESC
            LIMIT 1;
            ",
            &[&date, &ref_.remote, &ref_.name],
        )?;
    }
    for ref_ in out.changed.iter().chain(out.new.iter()) {
        let sha = git::get_sha(repository, &ref_.fullname())?;
        tx.execute(
            "
            INSERT INTO refs(remote, name, from_date, to_date, sha, tag)
            VALUES(?, ?, ?, NULL, ?, ?);
            ",
            &[&ref_.remote, &ref_.name, &date, &sha, &ref_.tag as &ToSql],
        )?;
    }

    // Create refs to prevent garbage collection
    for ref_ in out.changed.iter().chain(out.new.iter()) {
        let sha = git::get_sha(repository, &ref_.fullname())?;
        git::make_branch(repository, &format!("keep-{}", sha), &sha)?;
    }

    // Remove superfluous branches
    for ref_ in out.changed.iter().chain(out.new.iter()) {
        let sha = git::get_sha(repository, &ref_.fullname())?;
        let keeper = format!("keep-{}", sha);
        for br in git::included_branches(repository, &sha)? {
            if br != keeper {
                git::delete_branch(repository, &br)?;
            }
        }
        if git::including_branches(repository, &sha)?.len() > 1 {
            git::delete_branch(repository, &keeper)?;
        }
    }

    tx.commit()?;

    Ok(())
}
