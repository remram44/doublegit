use std::fmt;

use super::{
    Result as ApiResult, IssueRecorder,
    ProjectEnumerator, GitProject, Bugtracker,
};

pub struct Github {
    api_path: String,
    git_path: String,
}

impl Github {
    pub fn github_com() -> Github {
        Github {
            api_path: "https://api.github.com".into(),
            git_path: "https://github.com".into(),
        }
    }
}

impl ProjectEnumerator for Github {
    type Project = GithubProject;

    fn list_own_projects(username: &str) -> Vec<Self::Project> {
        // https://api.github.com/users/remram44/repos
        unimplemented!()
    }

    fn list_starred_projects(uesrname: &str) -> Vec<Self::Project> {
        // https://api.github.com/users/remram44/starred
        unimplemented!()
    }
}

pub struct GithubProject {
    url: String,
}

impl fmt::Display for GithubProject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.url)
    }
}

impl GitProject for GithubProject {
    fn git_url(&self) -> Option<String> {
        Some(format!("{}.git", self.url))
    }
}

impl Bugtracker for GithubProject {
    fn get_issues<Rec: IssueRecorder>(
        &self,
        recorder: Rec,
        last: Option<String>,
    ) -> ApiResult<()> {
        // https://api.github.com/repos/remram44/adler32-rs/issues
        // https://api.github.com/repos/remram44/adler32-rs/issues/events
        unimplemented!()
    }
}
