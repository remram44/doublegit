extern crate chrono;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate regex;
extern crate rusqlite;
#[cfg(feature = "web")] #[macro_use] extern crate serde;
#[cfg(feature = "web")] #[macro_use] extern crate serde_json;
#[cfg(test)] extern crate tempfile;

use rusqlite::Connection;
use rusqlite::types::ToSql;
use std::borrow::Cow;
use std::fmt;
use std::path::Path;
use std::time::SystemTime;

mod git;
#[cfg(feature = "web")] pub mod web;

#[cfg(test)] mod tests_integration;

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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Operation {
    FastForward,
    Forced,
    Pruned,
    Tag,
    New,
    Reject,
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ref {
    name: String,
    tag: bool,
}

impl Ref {
    fn parse_remote_ref(refname: &str) -> Result<Ref, Error> {
        let idx = refname.find('/').ok_or(Error::git("Invalid remote ref"))?;
        let remote = &refname[0..idx];
        if remote != "origin" {
            return Err(Error::git("Remote ref has invalid remote"));
        }
        let name = &refname[idx + 1..];
        Ok(Ref { name: name.into(), tag: false })
    }

    fn fullname(&self) -> Cow<String> {
        if self.tag {
            Cow::Borrowed(&self.name)
        } else {
            Cow::Owned(format!("origin/{}", self.name))
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
            WHERE name=?
            ORDER BY from_date DESC
            LIMIT 1;
            ",
            &[&date, &ref_.name],
        )?;
    }
    for ref_ in out.changed.iter().chain(out.new.iter()) {
        let sha = git::get_sha(repository, &ref_.fullname())?;
        tx.execute(
            "
            INSERT INTO refs(name, from_date, to_date, sha, tag)
            VALUES(?, ?, NULL, ?, ?);
            ",
            &[&ref_.name, &date, &sha, &ref_.tag as &ToSql],
        )?;
    }

    // Create refs to prevent garbage collection
    for ref_ in out.changed.iter().chain(out.new.iter()) {
        let sha = git::get_sha(repository, &ref_.fullname())?;
        if ref_.tag && git::is_annotated_tag(repository, &sha)? {
            info!("{:?} making ref {}", ref_, sha);
            git::make_ref(
                repository,
                &format!("refs/kept-tags/tag-{}", sha),
                &sha,
            )?;
        } else {
            info!("{:?} making branch {}", ref_, sha);
            git::make_branch(repository, &format!("keep-{}", sha), &sha)?;
        }
    }

    // Remove superfluous branches
    for ref_ in out.changed.iter().chain(out.new.iter()) {
        let sha = git::get_sha(repository, &ref_.fullname())?;
        let keeper = format!("keep-{}", sha);
        // Parents of this branch are superfluous
        for br in git::included_branches(repository, &sha)? {
            if br != keeper {
                git::delete_branch(repository, &br)?;
            }
        }
        // This branch is superfluous if it is included in others
        // If the ref is an annotated tag, this wrongly checks if the commit
        // is included in other branches, so skip on annotated tags
        if !(ref_.tag && git::is_annotated_tag(repository, &sha)?)
            && git::including_branches(repository, &sha)?.len() > 1
        {
            git::delete_branch(repository, &keeper)?;
        }
    }

    tx.commit()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::Ref;

    #[test]
    fn test_ref_parse() {
        assert_eq!(
            Ref::parse_remote_ref("origin/master").unwrap(),
            Ref {
                name: "master".into(),
                tag: false,
            },
        );
        assert!(Ref::parse_remote_ref("upstream/master").is_err());
        assert!(Ref::parse_remote_ref("master").is_err());
    }

    #[test]
    fn test_ref_fullname() {
        assert_eq!(
            &Ref {
                name: "master".into(),
                tag: false,
            }
            .fullname() as &str,
            "origin/master",
        );
        assert_eq!(
            &Ref {
                name: "release".into(),
                tag: true,
            }
            .fullname() as &str,
            "release",
        );
    }
}
