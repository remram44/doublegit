mod github;

enum Error {
    Io(std::io::Error),
}

type Result<T> = std::result::Result<T, Error>;

trait ProjectEnumerator {
    type Project;

    fn list_own_projects(username: &str) -> Vec<Self::Project> {
        Vec::new()
    }

    fn list_starred_projects(uesrname: &str) -> Vec<Self::Project> {
        Vec::new()
    }
}

trait GitProject {
    fn git_url(&self) -> Option<String>;
}

trait Bugtracker {
    fn get_issues<Rec: IssueRecorder>(
        &self,
        recorder: Rec,
        last: Option<String>,
    ) -> Result<()>;
}

struct MergeRequest {
    /// The base or target of the merge request
    base: String,
    /// The head or source of the merge request
    head: String,
}

trait IssueRecorder {
    /// Record a new issue
    fn record_issue(
        &mut self,
        id: &str,
        title: &str,
        description: Option<&str>,
        merge_request: Option<MergeRequest>,
    ) -> Result<()>;

    /// Record a comment in an issue's thread
    fn record_comment(
        &mut self,
        issue_id: &str,
        id: Option<&str>,
        parent: Option<&str>,
        text: Option<&str>,
    ) -> Result<()>;
}
