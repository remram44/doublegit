mod github;
mod recorder;
mod registry;

use erased_serde::Serialize;
use serde_json::Value;
use std::fs::File;
use std::path::Path;

use crate::Error;
use self::registry::get_platform;
use self::recorder::Recorder;

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Error {
        Error::Http(e)
    }
}

/// A Git platform, from which we can get projects.
pub trait GitPlatform: Serialize {
    /// If supported, return a list of all projects owned by a user
    fn list_own_projects(
        &self,
        username: &str,
    ) -> Result<Vec<Box<dyn GitProject>>, Error> {
        Err(Error::NotSupported)
    }

    /// If supported, return a list of all projects starred/followed by a user
    fn list_starred_projects(
        &self,
        username: &str,
    ) -> Result<Vec<Box<dyn GitProject>>, Error> {
        Err(Error::NotSupported)
    }
}

/// A project on a Git platform
pub trait GitProject: Serialize {
    /// Get the Git URL for this project, if supported
    fn git_url(&self) -> Option<String>;

    /// Read the issues/merge requests from this project, if supported
    fn update(
        &self,
        recorder: &mut Recorder,
        last: Option<String>,
    ) -> Result<(), Error>;
}

pub fn update_with_date(
    path: &Path,
    date: chrono::DateTime<chrono::Utc>,
) -> std::result::Result<(), Error>
{
    // Open configuration file
    let config_file = path.join("doublegit.json");
    if !config_file.exists() {
        info!("Config file {:?} doesn't exist, skipping API update", path);
        return Ok(());
    }
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

    // Load as JSON
    let mut config: Value = serde_json::from_reader(file)
        .map_err(|e| Error::Config( format!("Error reading config: {}", e)))?;

    // Should be an object with a key 'type'
    let type_name = if let Value::Object(ref mut obj) = config {
        if let Some(Value::String(s)) = obj.remove("type") {
            s
        } else {
            return Err(Error::Config(
                "Config does not contain a key \"type\"".into()
            ));
        }
    } else {
        return Err(Error::Config("Config is not an object".into()));
    };

    // Get API from registry
    let loader = if let Some(loader) = get_platform(&type_name) {
        loader
    } else {
        return Err(Error::Config("No such platform: {}".into()));
    };

    // Load configuration object
    let project: Box<dyn GitProject> = loader.load_project(config)
        .map_err(|e| Error::Config(format!("{}", e)))?;

    // Fetch project API data
    let mut recorder = Recorder::open(path)?;
    let last_event = recorder.last_event()?;
    project.update(&mut recorder, last_event)?;

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
fn update_directory(path: &Path) -> std::result::Result<(), Error> {
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
        .map_err(|e| Error::Config(
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
