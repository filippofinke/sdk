use thiserror::Error;

#[derive(Error, Debug)]
pub enum GetProjectDirsError {
    #[error("Cannot find home directory (no HOME environment variable).")]
    NoHomeInEnvironment(),
}
