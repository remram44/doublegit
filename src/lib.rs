#[macro_use] extern crate log;

use std::collections::HashSet;
use std::convert::TryFrom;
use std::io::Read;
use std::path::Path;
use std::time::SystemTime;

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
    fn parse_remote_ref(refname: &str, remote: &str, tag: bool) -> Ref {
        unimplemented!()
    }
}

struct FetchOutput {
    new: HashSet<Ref>,
    changed: HashSet<Ref>,
    removed: HashSet<Ref>,
}

fn fetch(repository: &Path) -> FetchOutput {
    unimplemented!()
}

fn parse_fetch_output<R: Read>(output: R) -> FetchOutput {
    unimplemented!()
}

pub fn update(repository: &Path) -> Result<(), ()> {
    update_with_date(repository, SystemTime::now())
}

pub fn update_with_date(
    repository: &Path,
    date: SystemTime,
) -> Result<(), ()> {
    info!("Updating {:?}...", repository);
    unimplemented!()
}
