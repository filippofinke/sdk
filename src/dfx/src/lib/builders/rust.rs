use crate::lib::builders::{
    BuildConfig, BuildOutput, CanisterBuilder, IdlBuildOutput, WasmBuildOutput,
};
use crate::lib::canister_info::rust::RustCanisterInfo;
use crate::lib::canister_info::CanisterInfo;
use crate::lib::environment::Environment;
use crate::lib::error::DfxResult;
use crate::lib::models::canister::CanisterPool;

use crate::lib::wasm::metadata::add_candid_service_metadata;
use anyhow::{anyhow, bail, Context};
use fn_error_context::context;
use ic_types::principal::Principal as CanisterId;
use serde::Deserialize;
use slog::{info, o, warn};
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

pub struct RustBuilder {
    logger: slog::Logger,
}

impl RustBuilder {
    #[context("Failed to create RustBuilder.")]
    pub fn new(env: &dyn Environment) -> DfxResult<Self> {
        Ok(RustBuilder {
            logger: env.get_logger().new(o! {
                "module" => "rust"
            }),
        })
    }
}

impl CanisterBuilder for RustBuilder {
    #[context("Failed to get dependencies for canister '{}'.", info.get_name())]
    fn get_dependencies(
        &self,
        pool: &CanisterPool,
        info: &CanisterInfo,
    ) -> DfxResult<Vec<CanisterId>> {
        let dependencies = info.get_dependencies()
            .iter()
            .map(|name| {
                pool.get_first_canister_with_name(name)
                    .map(|c| c.canister_id())
                    .map_or_else(
                        || Err(anyhow!("A canister with the name '{}' was not found in the current project.", name.clone())),
                        DfxResult::Ok,
                    )
            })
            .collect::<DfxResult<Vec<CanisterId>>>().with_context(|| format!("Failed to collect dependencies (canister ids) for canister {}.", info.get_name()))?;
        Ok(dependencies)
    }

    #[context("Failed to build Rust canister '{}'.", canister_info.get_name())]
    fn build(
        &self,
        pool: &CanisterPool,
        canister_info: &CanisterInfo,
        config: &BuildConfig,
    ) -> DfxResult<BuildOutput> {
        let rust_info = canister_info.as_info::<RustCanisterInfo>()?;
        let package = rust_info.get_package();

        let canister_id = canister_info.get_canister_id().unwrap();

        let mut cargo = Command::new("cargo");
        cargo
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .arg("build")
            .arg("--target")
            .arg("wasm32-unknown-unknown")
            .arg("--release")
            .arg("-p")
            .arg(package);

        let dependencies = self
            .get_dependencies(pool, canister_info)
            .unwrap_or_default();
        let vars =
            super::environment_variables(canister_info, &config.network_name, pool, &dependencies);
        for (key, val) in vars {
            cargo.env(key.as_ref(), val);
        }

        info!(
            self.logger,
            "Executing: cargo build --target wasm32-unknown-unknown --release -p {}", package
        );
        let output = cargo.output().context("Failed to run 'cargo build'.")?;

        info!(self.logger, "Optimizing WASM module.");
        let wasm_path = rust_info.get_output_wasm_path();
        let wasm = std::fs::read(wasm_path).expect("Could not read the WASM module.");
        let wasm_optimized =
            ic_wasm::optimize::optimize(&wasm).map_err(|e| anyhow!(e.to_string()))?;
        // The optimized wasm overwrites the original wasm.
        // Because the `get_output_wasm_path` must give the same path,
        // no matter optimized or not.
        std::fs::write(wasm_path, wasm_optimized).expect("Could not write optimized WASM module.");

        if !output.status.success() {
            bail!("Failed to compile the rust package: {}", package);
        }

        add_candid_service_metadata(
            rust_info.get_output_wasm_path(),
            rust_info.get_output_idl_path(),
        )?;

        Ok(BuildOutput {
            canister_id,
            wasm: WasmBuildOutput::File(rust_info.get_output_wasm_path().to_path_buf()),
            idl: IdlBuildOutput::File(rust_info.get_output_idl_path().to_path_buf()),
        })
    }

    fn generate_idl(
        &self,
        _pool: &CanisterPool,
        info: &CanisterInfo,
        _config: &BuildConfig,
    ) -> DfxResult<PathBuf> {
        let rust_info = info.as_info::<RustCanisterInfo>()?;
        let output_idl_path = rust_info.get_output_idl_path();
        if output_idl_path.exists() {
            Ok(output_idl_path.to_path_buf())
        } else {
            bail!(
                "Candid file: {} doesn't exist.",
                output_idl_path.to_string_lossy()
            );
        }
    }
}