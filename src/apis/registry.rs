use std::collections::HashMap;

use crate::Error;
use super::{GitPlatform, GitProject};

pub trait Loader: Sync {
    fn load_platform(&self, config: serde_json::Value) -> Result<Box<dyn GitPlatform>, Error>;

    fn load_project(&self, config: serde_json::Value) -> Result<Box<dyn GitProject>, Error>;
}

pub fn get_platform(name: &str) -> Option<Box<dyn Loader>> {
    unimplemented!()
}

lazy_static! {
    static ref LOADERS: HashMap<&'static str, Box<dyn Loader>> = {
        let mut map = HashMap::new();
        use super::github::GithubLoader;
        map.insert("github".into(), Box::new(GithubLoader) as Box<dyn Loader>);
        map
    };
}

// Construct from command-line
// Construct from on-disk repo
