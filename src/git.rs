use std::collections::HashSet;
use std::fmt;
use std::io::Read;
use std::path::Path;

use crate::{Operation, Ref};

#[derive(Debug)]
pub struct GitError(String);

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for GitError {}

fn parse_operation(chr: u8) -> Result<Operation, GitError> {
    Ok(match chr {
        b' ' => Operation::FastForward,
        b'+' => Operation::Forced,
        b'-' => Operation::Pruned,
        b't' => Operation::Tag,
        b'*' => Operation::New,
        b'!' => Operation::Reject,
        b'=' => Operation::Noop,
        _ => return Err(GitError("Parse error: invalid operation".into())),
    })
}

pub struct FetchOutput {
    pub new: HashSet<Ref>,
    pub changed: HashSet<Ref>,
    pub removed: HashSet<Ref>,
}

pub fn fetch(repository: &Path) -> Result<FetchOutput, GitError> {
    unimplemented!()
}

fn parse_fetch_output<R: Read>(output: R) -> Result<FetchOutput, GitError> {
    unimplemented!()
}

pub fn get_sha(repository: &Path, refname: &str) -> Result<String, GitError> {
    unimplemented!()
}

pub fn make_branch(
    repository: &Path,
    name: &str,
    sha: &str,
) -> Result<(), GitError> {
    unimplemented!()
}

pub fn included_branches(
    repository: &Path, target: &str,
) -> Result<Vec<String>, GitError> {
    unimplemented!()
}

pub fn including_branches(
    repository: &Path,
    target: &str,
) -> Result<Vec<String>, GitError> {
    unimplemented!()
}

pub fn delete_branch(repository: &Path, name: &str) -> Result<(), GitError> {
    unimplemented!()
}
