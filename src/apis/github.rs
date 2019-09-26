use reqwest;
use serde_json::Value;
use std::fmt;

use crate::Error;
use super::{GitPlatform, GitProject, Recorder};

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

    fn update(
        &self,
        recorder: &mut Recorder,
        last: Option<String>,
    ) -> Result<(), Error>
    {
        let response = reqwest::get(&format!(
            "{}/repos/{}/issues/events",
            self.platform.api_path,
            self.url,
        ))?.json()?;
        let events = if let Value::Array(a) = response {
            a
        } else {
            return Err(Error::Api(
                "GitHub API events are not an array".into()
            ));
        };
        for event in events {
            // TODO: record event, parse events of interest (issue/pr)
            unimplemented!()
        }
        //recorder.set_last_event(TODO)?;
        Ok(())
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
