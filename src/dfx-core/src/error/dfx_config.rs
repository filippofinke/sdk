use thiserror::Error;

#[derive(Error, Debug)]
pub enum DfxConfigError {
    #[error("Circular canister dependencies: {}", _0.join(" -> "))]
    CanisterCircularDependency(Vec<String>),

    #[error("Canister '{0}' not found.")]
    CanisterNotFound(String),

    #[error("No canisters in the configuration file.")]
    CanistersFieldDoesNotExist(),

    #[error("Failed to get canisters with their dependencies (for {}): {1}", _0.as_deref().unwrap_or("all canisters"))]
    GetCanistersWithDependenciesFailed(Option<String>, Box<DfxConfigError>),
}
