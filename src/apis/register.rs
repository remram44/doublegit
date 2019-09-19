use super::{Result as ApiResult, GitPlatform, GitProject};

trait Loader {
    fn load_platforn(&self, reader: &[u8]) -> ApiResult<Box<dyn GitPlatform>>;

    fn load_project(&self, reader: &[u8]) -> ApiResult<Box<dyn GitProject>>;
}

pub struct Registry {
}

impl Registry {
    fn register_platform(
        name: &str,
        loader: Box<dyn Loader>,
    ) {
    }
}

lazy_static! {
    pub static ref registry: Registry = Registry {};
}

// Construct from command-line
// Construct from on-disk repo
