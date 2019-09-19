use erased_serde::Serialize;
//use std::fs::File;
use std::path::Path;

mod github;
mod register;

pub enum Error {
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
    ) -> Result<Vec<Box<dyn GitProject>>> {
        Err(Error::NotSupported)
    }

    /// If supported, return a list of all projects starred/followed by a user
    fn list_starred_projects(
        &self,
        username: &str,
    ) -> Result<Vec<Box<dyn GitProject>>> {
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

pub fn update_project(path: &Path) -> std::result::Result<(), crate::Error> {
    Ok(())
}

/*
/// Config file, either for a project or a collection of projects.
#[derive(Serialize, Deserialize)]
enum EitherConfig {
    Project(serde_json::Value),
    Collection(serde_json::Value),
}

/// Update a directory, which is either a project or a collection of projects.
fn update_directory(path: &Path) -> std::result::Result<(), crate::Error> {
    info!("Updating directory {:?}...", path);
    let config_file = path.join("doublegit.json");
    let file = match File::open(&config_file) {
        Ok(f) => {
            info!("Loaded config file {:?}", config_file);
            f
        }
        Err(e) => {
            warn!("Couldn't open config file {:?}", config_file);
            return Err(e.into());
        }
    };
    let config: EitherConfig = serde_json::from_reader(file)
        .map_err(|e| crate::Error::Config(
            format!("Error reading config: {}", e)
        ))?;
    match config {
        EitherConfig::Project(config) => {
            info!("Config is project, updating");
            let project = register::registry.load_;
            unimplemented!();
        }
        EitherConfig::Collection(config) => {
            info!("Config is a collection, updating");
            unimplemented!()
        }
    }
    Ok(())
}
*/
