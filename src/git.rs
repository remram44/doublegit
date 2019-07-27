use std::collections::HashSet;
use std::io::Read;
use std::path::Path;

use crate::{Error, Operation, Ref};

fn parse_operation(chr: u8) -> Result<Operation, Error> {
    Ok(match chr {
        b' ' => Operation::FastForward,
        b'+' => Operation::Forced,
        b'-' => Operation::Pruned,
        b't' => Operation::Tag,
        b'*' => Operation::New,
        b'!' => Operation::Reject,
        b'=' => Operation::Noop,
        _ => return Err(Error::git("Parse error: invalid operation")),
    })
}

pub struct FetchOutput {
    pub new: HashSet<Ref>,
    pub changed: HashSet<Ref>,
    pub removed: HashSet<Ref>,
}

pub fn fetch(repository: &Path) -> Result<FetchOutput, Error> {
    unimplemented!()
}

fn parse_fetch_output<R: Read>(output: R) -> Result<FetchOutput, Error> {
    unimplemented!()
}

pub fn get_sha(repository: &Path, refname: &str) -> Result<String, Error> {
    unimplemented!()
}

pub fn make_branch(
    repository: &Path,
    name: &str,
    sha: &str,
) -> Result<(), Error> {
    unimplemented!()
}

pub fn included_branches(
    repository: &Path, target: &str,
) -> Result<Vec<String>, Error> {
    unimplemented!()
}

pub fn including_branches(
    repository: &Path,
    target: &str,
) -> Result<Vec<String>, Error> {
    unimplemented!()
}

pub fn delete_branch(repository: &Path, name: &str) -> Result<(), Error> {
    unimplemented!()
}
