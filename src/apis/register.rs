use super::{Result as ApiResult, GitPlatform, GitProject};

#[derive(Debug)]
pub struct Registry {
}

pub trait Loader {
    fn load_platforn(&self, reader: &[u8]) -> ApiResult<Box<GitPlatform>>;

    fn load_project(&self, reader: &[u8]) -> ApiResult<Box<GitProject>>;
}

impl Registry {
    fn new() -> Registry {
        Registry {
        }
    }

    pub fn register_platform(
        name: &str,
        loader: Box<Loader>,
    ) {
    }
}

lazy_static! {
    pub static ref registry: Registry = Registry::new();
}

// Construct from command-line
// Construct from on-disk repo
