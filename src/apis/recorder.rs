use rusqlite;
use serde_json::Value;
use std::path::Path;

use super::Error;

/// Represent merge request information, that may be attached to issues
pub struct MergeRequest {
    /// The base or target of the merge request
    pub base: String,
    /// The head or source of the merge request
    pub head: String,
}

/// Recorder object through which `GitProject::update()` can save information
pub struct Recorder {
}

impl Recorder {
    /// Record a generic unparsed event in native format
    pub fn record_event(
        &mut self,
        id: &str,
        type_: &str,
        date: &str,
        event: Value
    ) -> Result<(), Error>
    {
        unimplemented!()
    }

    /// Record a new issue
    pub fn record_issue(
        &mut self,
        id: &str,
        date: &str,
        title: &str,
        description: Option<&str>,
        merge_request: Option<MergeRequest>,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    /// Record a comment in an issue's thread
    pub fn record_comment(
        &mut self,
        issue_id: &str,
        id: Option<&str>,
        parent: Option<&str>,
        date: &str,
        text: Option<&str>,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    pub(crate) fn open(path: &Path) -> Result<Recorder, Error> {
        unimplemented!()
    }

    pub(crate) fn last_event(&self) -> Result<Option<String>, Error> {
        unimplemented!()
    }
}
