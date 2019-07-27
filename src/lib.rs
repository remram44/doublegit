extern crate chrono;
#[macro_use] extern crate log;
extern crate rusqlite;

use rusqlite::Connection;
use rusqlite::types::ToSql;
use std::borrow::Cow;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::fmt;
use std::io::Read;
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug)]
pub enum Error {
    Sqlite(rusqlite::Error),
    Git(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Sqlite(e) => write!(f, "SQLite error: {}", e),
            Error::Git(e) => write!(f, "Git error: {}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Error {
        Error::Sqlite(e)
    }
}

enum Operation {
    FastForward,
    Forced,
    Pruned,
    Tag,
    New,
    Reject,
    Noop,
}

impl TryFrom<u8> for Operation {
    type Error = ();

    fn try_from(chr: u8) -> Result<Operation, Self::Error> {
        Ok(match chr {
            b' ' => Operation::FastForward,
            b'+' => Operation::Forced,
            b'-' => Operation::Pruned,
            b't' => Operation::Tag,
            b'*' => Operation::New,
            b'!' => Operation::Reject,
            b'=' => Operation::Noop,
            _ => return Err(()),
        })
    }
}

pub struct Ref {
    remote: String,
    name: String,
    tag: bool,
}

impl Ref {
    fn parse_remote_ref(
        refname: &str,
        remote: &str,
        tag: bool,
    ) -> Result<Ref, ()> {
        unimplemented!()
    }

    fn fullname(&self) -> Cow<String> {
        if self.tag {
            Cow::Borrowed(&self.name)
        } else {
            Cow::Owned(format!("{}/{}", self.remote, self.name))
        }
    }
}

struct FetchOutput {
    new: HashSet<Ref>,
    changed: HashSet<Ref>,
    removed: HashSet<Ref>,
}

fn fetch(repository: &Path) -> Result<FetchOutput, Error> {
    unimplemented!()
}

fn parse_fetch_output<R: Read>(output: R) -> Result<FetchOutput, Error> {
    unimplemented!()
}

fn get_sha(repository: &Path, refname: &str) -> Result<String, Error> {
    unimplemented!()
}

fn make_branch(repository: &Path, name: &str, sha: &str) -> Result<(), Error> {
    unimplemented!()
}

fn included_branches(
    repository: &Path, target: &str,
) -> Result<Vec<String>, Error> {
    unimplemented!()
}

fn including_branches(
    repository: &Path,
    target: &str,
) -> Result<Vec<String>, Error> {
    unimplemented!()
}

fn delete_branch(repository: &Path, name: &str) -> Result<(), Error> {
    unimplemented!()
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
    let out = fetch(repository)?;

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
        let sha = get_sha(repository, &ref_.fullname())?;
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
        let sha = get_sha(repository, &ref_.fullname())?;
        make_branch(repository, &format!("keep-{}", sha), &sha)?;
    }

    // Remove superfluous branches
    for ref_ in out.changed.iter().chain(out.new.iter()) {
        let sha = get_sha(repository, &ref_.fullname())?;
        let keeper = format!("keep-{}", sha);
        for br in included_branches(repository, &sha)? {
            if br != keeper {
                delete_branch(repository, &br)?;
            }
        }
        if including_branches(repository, &sha)?.len() > 1 {
            delete_branch(repository, &keeper)?;
        }
    }

    tx.commit()?;

    Ok(())
}
