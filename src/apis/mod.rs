use erased_serde::Serialize;

mod github;
mod register;

enum Error {
    Io(std::io::Error),
    NotSupported,
}

type Result<T> = std::result::Result<T, Error>;

/// A Git platform, from which we can get projects.
trait GitPlatform: Serialize {
    /// If supported, return a list of all projects owned by a user
    fn list_own_projects(
        &self,
        username: &str,
    ) -> Result<Vec<Box<GitProject>>> {
        Err(Error::NotSupported)
    }

    /// If supported, return a list of all projects starred/followed by a user
    fn list_starred_projects(
        &self,
        username: &str,
    ) -> Result<Vec<Box<GitProject>>> {
        Err(Error::NotSupported)
    }
}

/// A project on a Git platform
trait GitProject: Serialize {
    /// Get the Git URL for this project, if supported
    fn git_url(&self) -> Option<String>;

    /// Read the issues/merge requests from this project, if supported
    fn get_issues(
        &self,
        recorder: IssueRecorder,
        last: Option<String>,
    ) -> Result<()>;
}

/// Represent merge request information, that may be attached to issues
pub struct MergeRequest {
    /// The base or target of the merge request
    pub base: String,
    /// The head or source of the merge request
    pub head: String,
}

/// Recorder object through which `GitProject::get_issues()` can record issues
pub struct IssueRecorder {
}

impl IssueRecorder {
    /// Record a new issue
    pub fn record_issue(
        &mut self,
        id: &str,
        title: &str,
        description: Option<&str>,
        merge_request: Option<MergeRequest>,
    ) -> Result<()> {
        unimplemented!()
    }

    /// Record a comment in an issue's thread
    pub fn record_comment(
        &mut self,
        issue_id: &str,
        id: Option<&str>,
        parent: Option<&str>,
        text: Option<&str>,
    ) -> Result<()> {
        unimplemented!()
    }
}
