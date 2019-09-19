use std::fmt;

use crate::Error;
use super::{IssueRecorder, GitPlatform, GitProject};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    pub fn github_enterprise(api_path: &str, git_path: &str) -> Github {
        Github {
            api_path: api_path.into(),
            git_path: git_path.into(),
        }
    }
}

impl GitPlatform for Github {
    fn list_own_projects(
        &self,
        username: &str,
    ) -> Result<Vec<Box<dyn GitProject>>, Error> {
        // https://api.github.com/users/remram44/repos
        unimplemented!()
    }

    fn list_starred_projects(
        &self,
        username: &str,
    ) -> Result<Vec<Box<dyn GitProject>>, Error> {
        // https://api.github.com/users/remram44/starred
        unimplemented!()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubProject {
    platform: Github,
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

    fn get_issues(
        &self,
        recorder: IssueRecorder,
        last: Option<String>,
    ) -> Result<(), Error> {
        // https://api.github.com/repos/remram44/adler32-rs/issues
        // https://api.github.com/repos/remram44/adler32-rs/issues/events
        unimplemented!()
    }
}

pub struct GithubLoader;

impl super::registry::Loader for GithubLoader {
    fn load_platform(
        &self,
        config: serde_json::Value,
    ) -> Result<Box<dyn GitPlatform>, Error> {
        let gh: Github = serde_json::from_value(config)
            .map_err(|e| Error::Config(
                format!("Invalid configuration: {}", e)
            ))?;
        Ok(Box::new(gh))
    }

    fn load_project(
        &self,
        config: serde_json::Value,
    ) -> Result<Box<dyn GitProject>, Error> {
        let proj: GithubProject = serde_json::from_value(config)
            .map_err(|e| Error::Config(
                format!("Invalid configuration: {}", e)
            ))?;
        Ok(Box::new(proj))
    }
}
