#![allow(dead_code)]
use crate::lib::bitcoin::adapter::config::BitcoinAdapterLogLevel;
use crate::lib::error::{BuildError, DfxError, DfxResult};
use crate::util::{PossiblyStr, SerdeVec};
use crate::{error_invalid_argument, error_invalid_config, error_invalid_data};

use anyhow::{anyhow, Context};
use byte_unit::Byte;
use fn_error_context::context;
use ic_types::Principal;
use serde::de::{Error as _, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::default::Default;
use std::fmt;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const CONFIG_FILE_NAME: &str = "dfx.json";

const EMPTY_CONFIG_DEFAULTS: ConfigDefaults = ConfigDefaults {
    bitcoin: None,
    bootstrap: None,
    build: None,
    canister_http: None,
    replica: None,
};

pub const EMPTY_CONFIG_DEFAULTS_BITCOIN: ConfigDefaultsBitcoin = ConfigDefaultsBitcoin {
    enabled: false,
    nodes: None,
    log_level: BitcoinAdapterLogLevel::Info,
};

pub const EMPTY_CONFIG_DEFAULTS_CANISTER_HTTP: ConfigDefaultsCanisterHttp =
    ConfigDefaultsCanisterHttp { enabled: false };

pub const EMPTY_CONFIG_DEFAULTS_BOOTSTRAP: ConfigDefaultsBootstrap = ConfigDefaultsBootstrap {
    ip: None,
    port: None,
    timeout: None,
};

const EMPTY_CONFIG_DEFAULTS_BUILD: ConfigDefaultsBuild = ConfigDefaultsBuild {
    packtool: None,
    args: None,
};

pub const EMPTY_CONFIG_DEFAULTS_REPLICA: ConfigDefaultsReplica = ConfigDefaultsReplica {
    port: None,
    subnet_type: None,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConfigCanistersCanisterRemote {
    pub candid: Option<PathBuf>,

    // network -> canister ID
    pub id: BTreeMap<String, Principal>,
}

const DEFAULT_LOCAL_BIND: &str = "127.0.0.1:8000";
pub const DEFAULT_IC_GATEWAY: &str = "https://ic0.app";
pub const DEFAULT_IC_GATEWAY_TRAILING_SLASH: &str = "https://ic0.app/";

/// A Canister configuration in the dfx.json config file.
/// It only contains a type; everything else should be infered using the
/// CanisterInfo type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigCanistersCanister {
    #[serde(default)]
    pub declarations: CanisterDeclarationsConfig,

    #[serde(default)]
    pub remote: Option<ConfigCanistersCanisterRemote>,

    pub args: Option<String>,

    #[serde(default)]
    pub initialization_values: InitializationValues,

    #[serde(default)]
    pub dependencies: Vec<String>,

    pub frontend: Option<BTreeMap<String, String>>,

    #[serde(flatten)]
    pub type_specific: CanisterTypeProperties,

    #[serde(default)]
    pub post_install: SerdeVec<String>,
    pub main: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CanisterTypeProperties {
    Rust {
        package: String,
        candid: PathBuf,
    },
    Assets {
        source: Vec<PathBuf>,
    },
    Custom {
        wasm: PathBuf,
        candid: PathBuf,
        build: SerdeVec<String>,
    },
    Motoko,
}

impl CanisterTypeProperties {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Rust { .. } => "rust",
            Self::Motoko { .. } => "motoko",
            Self::Assets { .. } => "assets",
            Self::Custom { .. } => "custom",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct InitializationValues {
    pub compute_allocation: Option<PossiblyStr<u64>>,
    pub memory_allocation: Option<Byte>,
    #[serde(with = "humantime_serde")]
    pub freezing_threshold: Option<Duration>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CanisterDeclarationsConfig {
    // Directory to place declarations for that canister
    // Default is "src/declarations/<canister_name>"
    pub output: Option<PathBuf>,

    // A list of languages to generate type declarations
    // Supported options are "js", "ts", "did", "mo"
    // default is ["js", "ts", "did"]
    pub bindings: Option<Vec<String>>,

    // A string that will replace process.env.{canister_name_uppercase}_CANISTER_ID
    // in the "src/dfx/assets/language_bindings/canister.js" template
    pub env_override: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConfigDefaultsBitcoin {
    #[serde(default)]
    pub enabled: bool,

    /// Addresses of nodes to connect to (in case discovery from seeds is not possible/sufficient)
    #[serde(default)]
    pub nodes: Option<Vec<SocketAddr>>,

    /// The logging level of the adapter (e.g. "info", "debug", "error", etc.)
    #[serde(default)]
    pub log_level: BitcoinAdapterLogLevel,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ConfigDefaultsCanisterHttp {
    #[serde(default)]
    pub enabled: bool,
}

fn default_as_false() -> bool {
    // sigh https://github.com/serde-rs/serde/issues/368
    false
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ConfigDefaultsBootstrap {
    pub ip: Option<IpAddr>,
    pub port: Option<u16>,
    pub timeout: Option<u64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConfigDefaultsBuild {
    pub packtool: Option<String>,
    pub args: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ConfigDefaultsReplica {
    pub port: Option<u16>,
    pub subnet_type: Option<ReplicaSubnetType>,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkType {
    // We store ephemeral canister ids in .dfx/{network}/canister_ids.json
    Ephemeral,

    // We store persistent canister ids in canister_ids.json (adjacent to dfx.json)
    Persistent,
}

impl Default for NetworkType {
    // This is just needed for the Default trait on NetworkType,
    // but nothing will ever call it, due to field defaults.
    fn default() -> Self {
        NetworkType::Ephemeral
    }
}

impl NetworkType {
    fn ephemeral() -> Self {
        NetworkType::Ephemeral
    }
    fn persistent() -> Self {
        NetworkType::Persistent
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReplicaSubnetType {
    System,
    Application,
    VerifiedApplication,
}

impl Default for ReplicaSubnetType {
    fn default() -> Self {
        ReplicaSubnetType::Application
    }
}

impl ReplicaSubnetType {
    /// Converts the value to the string expected by ic-starter for its --subnet-type argument
    pub fn as_ic_starter_string(&self) -> String {
        match self {
            ReplicaSubnetType::System => "system".to_string(),
            ReplicaSubnetType::Application => "application".to_string(),
            ReplicaSubnetType::VerifiedApplication => "verified_application".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfigNetworkProvider {
    pub providers: Vec<String>,

    #[serde(default = "NetworkType::persistent")]
    pub r#type: NetworkType,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ConfigLocalProvider {
    pub bind: String,

    #[serde(default = "NetworkType::ephemeral")]
    pub r#type: NetworkType,

    pub bitcoin: Option<ConfigDefaultsBitcoin>,
    pub bootstrap: Option<ConfigDefaultsBootstrap>,
    pub canister_http: Option<ConfigDefaultsCanisterHttp>,
    pub replica: Option<ConfigDefaultsReplica>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ConfigNetwork {
    ConfigNetworkProvider(ConfigNetworkProvider),
    ConfigLocalProvider(ConfigLocalProvider),
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Profile {
    // debug is for development only
    Debug,
    // release is for production
    Release,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConfigDefaults {
    pub bitcoin: Option<ConfigDefaultsBitcoin>,
    pub bootstrap: Option<ConfigDefaultsBootstrap>,
    pub build: Option<ConfigDefaultsBuild>,
    pub canister_http: Option<ConfigDefaultsCanisterHttp>,
    pub replica: Option<ConfigDefaultsReplica>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigInterface {
    pub profile: Option<Profile>,
    pub version: Option<u32>,
    pub dfx: Option<String>,
    pub canisters: Option<BTreeMap<String, ConfigCanistersCanister>>,
    pub defaults: Option<ConfigDefaults>,
    pub networks: Option<BTreeMap<String, ConfigNetwork>>,
}

impl ConfigCanistersCanister {}

#[context("Failed to convert '{}' to a SocketAddress.", s)]
pub fn to_socket_addr(s: &str) -> DfxResult<SocketAddr> {
    match s.to_socket_addrs() {
        Ok(mut a) => match a.next() {
            Some(res) => Ok(res),
            None => Err(DfxError::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Empty iterator",
            ))),
        },
        Err(err) => Err(DfxError::new(err)),
    }
}

impl ConfigDefaultsBuild {
    pub fn get_packtool(&self) -> Option<String> {
        match &self.packtool {
            Some(v) if !v.is_empty() => self.packtool.to_owned(),
            _ => None,
        }
    }
    pub fn get_args(&self) -> Option<String> {
        match &self.args {
            Some(v) if !v.is_empty() => self.args.to_owned(),
            _ => None,
        }
    }
}

impl ConfigDefaults {
    pub fn get_build(&self) -> &ConfigDefaultsBuild {
        match &self.build {
            Some(x) => x,
            None => &EMPTY_CONFIG_DEFAULTS_BUILD,
        }
    }
}

impl ConfigInterface {
    pub fn get_defaults(&self) -> &ConfigDefaults {
        match &self.defaults {
            Some(v) => v,
            _ => &EMPTY_CONFIG_DEFAULTS,
        }
    }

    pub fn get_network(&self, name: &str) -> Option<ConfigNetwork> {
        let network = self
            .networks
            .as_ref()
            .and_then(|networks| networks.get(name).cloned());
        match (name, &network) {
            ("local", None) => Some(ConfigNetwork::ConfigLocalProvider(ConfigLocalProvider {
                bind: String::from(DEFAULT_LOCAL_BIND),
                r#type: NetworkType::Ephemeral,
                bitcoin: None,
                bootstrap: None,
                canister_http: None,
                replica: None,
            })),
            ("ic", _) => Some(ConfigNetwork::ConfigNetworkProvider(
                ConfigNetworkProvider {
                    providers: vec![DEFAULT_IC_GATEWAY.to_string()],
                    r#type: NetworkType::Persistent,
                },
            )),
            _ => network,
        }
    }

    pub fn get_version(&self) -> u32 {
        self.version.unwrap_or(1)
    }
    pub fn get_dfx(&self) -> Option<String> {
        self.dfx.to_owned()
    }

    /// Return the names of the specified canister and all of its dependencies.
    /// If none specified, return the names of all canisters.
    #[context("Failed to get canisters with their dependencies (for {}).", some_canister.unwrap_or("all canisters"))]
    pub fn get_canister_names_with_dependencies(
        &self,
        some_canister: Option<&str>,
    ) -> DfxResult<Vec<String>> {
        let canister_map = (&self.canisters)
            .as_ref()
            .ok_or_else(|| error_invalid_config!("No canisters in the configuration file."))?;

        let canister_names = match some_canister {
            Some(specific_canister) => {
                let mut names = HashSet::new();
                let mut path = vec![];
                add_dependencies(canister_map, &mut names, &mut path, specific_canister)?;
                names.into_iter().collect()
            }
            None => canister_map.keys().cloned().collect(),
        };

        Ok(canister_names)
    }

    #[context(
        "Failed to figure out if canister '{}' has a remote id on network '{}'.",
        canister,
        network
    )]
    pub fn get_remote_canister_id(
        &self,
        canister: &str,
        network: &str,
    ) -> DfxResult<Option<Principal>> {
        let maybe_principal = (&self.canisters)
            .as_ref()
            .ok_or_else(|| error_invalid_config!("No canisters in the configuration file."))?
            .get(canister)
            .ok_or_else(|| error_invalid_argument!("Canister {} not found in dfx.json", canister))?
            .remote
            .as_ref()
            .and_then(|r| r.id.get(network))
            .copied();
        Ok(maybe_principal)
    }

    #[context(
        "Failed while determining if canister '{}' is remote on network '{}'.",
        canister,
        network
    )]
    pub fn is_remote_canister(&self, canister: &str, network: &str) -> DfxResult<bool> {
        Ok(self.get_remote_canister_id(canister, network)?.is_some())
    }

    #[context("Failed to get compute allocation for '{}'.", canister_name)]
    pub fn get_compute_allocation(&self, canister_name: &str) -> DfxResult<Option<u64>> {
        Ok(self
            .get_canister_config(canister_name)?
            .initialization_values
            .compute_allocation
            .map(|x| x.0))
    }

    #[context("Failed to get memory allocation for '{}'.", canister_name)]
    pub fn get_memory_allocation(&self, canister_name: &str) -> DfxResult<Option<Byte>> {
        Ok(self
            .get_canister_config(canister_name)?
            .initialization_values
            .memory_allocation)
    }

    #[context("Failed to get freezing threshold for '{}'.", canister_name)]
    pub fn get_freezing_threshold(&self, canister_name: &str) -> DfxResult<Option<Duration>> {
        Ok(self
            .get_canister_config(canister_name)?
            .initialization_values
            .freezing_threshold)
    }

    fn get_canister_config(&self, canister_name: &str) -> DfxResult<&ConfigCanistersCanister> {
        let canister_map = self
            .canisters
            .as_ref()
            .ok_or_else(|| error_invalid_config!("No canisters in the configuration file."))?;

        let canister_config = canister_map
            .get(canister_name)
            .with_context(|| format!("Cannot find canister '{canister_name}'."))?;
        Ok(canister_config)
    }
}

#[context("Failed to add dependencies for canister '{}'.", canister_name)]
fn add_dependencies(
    all_canisters: &BTreeMap<String, ConfigCanistersCanister>,
    names: &mut HashSet<String>,
    path: &mut Vec<String>,
    canister_name: &str,
) -> DfxResult {
    let inserted = names.insert(String::from(canister_name));

    if !inserted {
        return if path.contains(&String::from(canister_name)) {
            path.push(String::from(canister_name));
            Err(DfxError::new(BuildError::DependencyError(format!(
                "Found circular dependency: {}",
                path.join(" -> ")
            ))))
        } else {
            Ok(())
        };
    }

    let canister_config = all_canisters
        .get(canister_name)
        .ok_or_else(|| anyhow!("Cannot find canister '{}'.", canister_name))?;

    path.push(String::from(canister_name));

    for canister in &canister_config.dependencies {
        add_dependencies(all_canisters, names, path, canister)?;
    }

    path.pop();

    Ok(())
}

#[derive(Clone)]
pub struct Config {
    path: PathBuf,
    json: Value,
    // public interface to the config:
    pub config: ConfigInterface,
}

#[allow(dead_code)]
impl Config {
    #[context("Failed to resolve config path from {}.", working_dir.to_string_lossy())]
    fn resolve_config_path(working_dir: &Path) -> DfxResult<Option<PathBuf>> {
        let mut curr = PathBuf::from(working_dir).canonicalize().with_context(|| {
            format!(
                "Failed to canonicalize working dir path {:}.",
                working_dir.to_string_lossy()
            )
        })?;
        while curr.parent().is_some() {
            if curr.join(CONFIG_FILE_NAME).is_file() {
                return Ok(Some(curr.join(CONFIG_FILE_NAME)));
            } else {
                curr.pop();
            }
        }

        // Have to check if the config could be in the root (e.g. on VMs / CI).
        if curr.join(CONFIG_FILE_NAME).is_file() {
            return Ok(Some(curr.join(CONFIG_FILE_NAME)));
        }

        Ok(None)
    }

    #[context("Failed to load config from {}.", path.to_string_lossy())]
    fn from_file(path: &Path) -> DfxResult<Config> {
        let content = std::fs::read(&path)
            .with_context(|| format!("Failed to read {}.", path.to_string_lossy()))?;
        Ok(Config::from_slice(path.to_path_buf(), &content)?)
    }

    #[context("Failed to read config from directory {}.", working_dir.to_string_lossy())]
    pub fn from_dir(working_dir: &Path) -> DfxResult<Option<Config>> {
        let path = Config::resolve_config_path(working_dir)?;
        let maybe_config = path.map(|path| Config::from_file(&path)).transpose()?;
        Ok(maybe_config)
    }

    #[context("Failed to read config from current working directory.")]
    pub fn from_current_dir() -> DfxResult<Option<Config>> {
        Config::from_dir(
            &std::env::current_dir().context("Failed to determine current working dir.")?,
        )
    }

    fn from_slice(path: PathBuf, content: &[u8]) -> std::io::Result<Config> {
        let config = serde_json::from_slice(content)?;
        let json = serde_json::from_slice(content)?;
        Ok(Config { path, json, config })
    }

    /// Create a configuration from a string.
    pub fn from_str(content: &str) -> std::io::Result<Config> {
        Config::from_slice(PathBuf::from("-"), content.as_bytes())
    }

    #[cfg(test)]
    pub fn from_str_and_path(path: PathBuf, content: &str) -> std::io::Result<Config> {
        Config::from_slice(path, content.as_bytes())
    }

    pub fn get_path(&self) -> &PathBuf {
        &self.path
    }
    pub fn get_temp_path(&self) -> PathBuf {
        self.get_path().parent().unwrap().join(".dfx")
    }
    pub fn get_json(&self) -> &Value {
        &self.json
    }
    pub fn get_mut_json(&mut self) -> &mut Value {
        &mut self.json
    }
    pub fn get_config(&self) -> &ConfigInterface {
        &self.config
    }

    pub fn get_project_root(&self) -> &Path {
        // a configuration path contains a file name specifically. As
        // such we should be returning at least root as parent. If
        // this is invariance is broken, we must fail.
        self.path.parent().expect(
            "An incorrect configuration path was set with no parent, i.e. did not include root",
        )
    }

    pub fn save(&self) -> DfxResult {
        let json_pretty = serde_json::to_string_pretty(&self.json)
            .map_err(|e| error_invalid_data!("Failed to serialize dfx.json: {}", e))?;
        std::fs::write(&self.path, json_pretty).with_context(|| {
            format!("Failed to write config to {}.", self.path.to_string_lossy())
        })?;
        Ok(())
    }
}

// grumble grumble https://github.com/serde-rs/serde/issues/2231
impl<'de> Deserialize<'de> for CanisterTypeProperties {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(PropertiesVisitor)
    }
}

struct PropertiesVisitor;

impl<'de> Visitor<'de> for PropertiesVisitor {
    type Value = CanisterTypeProperties;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("canister type metadata")
    }
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let missing_field = A::Error::missing_field;
        let (mut package, mut source, mut candid, mut build, mut wasm, mut r#type) =
            (None, None, None, None, None, None);
        while let Some(key) = map.next_key::<String>()? {
            match &*key {
                "package" => package = Some(map.next_value()?),
                "source" => source = Some(map.next_value()?),
                "candid" => candid = Some(map.next_value()?),
                "build" => build = Some(map.next_value()?),
                "wasm" => wasm = Some(map.next_value()?),
                "type" => r#type = Some(map.next_value::<String>()?),
                _ => continue,
            }
        }
        let props = match r#type.as_deref() {
            Some("motoko") | None => CanisterTypeProperties::Motoko,
            Some("rust") => CanisterTypeProperties::Rust {
                candid: candid.ok_or_else(|| missing_field("candid"))?,
                package: package.ok_or_else(|| missing_field("package"))?,
            },
            Some("assets") => CanisterTypeProperties::Assets {
                source: source.ok_or_else(|| missing_field("source"))?,
            },
            Some("custom") => CanisterTypeProperties::Custom {
                build: build.ok_or_else(|| missing_field("build"))?,
                candid: candid.ok_or_else(|| missing_field("candid"))?,
                wasm: wasm.ok_or_else(|| missing_field("wasm"))?,
            },
            Some(x) => {
                return Err(A::Error::unknown_variant(
                    x,
                    &["motoko", "rust", "assets", "custom"],
                ))
            }
        };
        Ok(props)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_dfinity_config_current_path() {
        let root_dir = tempfile::tempdir().unwrap();
        let root_path = root_dir.into_path().canonicalize().unwrap();
        let config_path = root_path.join("foo/fah/bar").join(CONFIG_FILE_NAME);

        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        std::fs::write(&config_path, "{}").unwrap();

        assert_eq!(
            config_path,
            Config::resolve_config_path(config_path.parent().unwrap())
                .unwrap()
                .unwrap(),
        );
    }

    #[test]
    fn find_dfinity_config_parent() {
        let root_dir = tempfile::tempdir().unwrap();
        let root_path = root_dir.into_path().canonicalize().unwrap();
        let config_path = root_path.join("foo/fah/bar").join(CONFIG_FILE_NAME);

        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        std::fs::write(&config_path, "{}").unwrap();

        assert!(
            Config::resolve_config_path(config_path.parent().unwrap().parent().unwrap())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn find_dfinity_config_subdir() {
        let root_dir = tempfile::tempdir().unwrap();
        let root_path = root_dir.into_path().canonicalize().unwrap();
        let config_path = root_path.join("foo/fah/bar").join(CONFIG_FILE_NAME);
        let subdir_path = config_path.parent().unwrap().join("baz/blue");

        std::fs::create_dir_all(&subdir_path).unwrap();
        std::fs::write(&config_path, "{}").unwrap();

        assert_eq!(
            config_path,
            Config::resolve_config_path(subdir_path.as_path())
                .unwrap()
                .unwrap(),
        );
    }

    #[test]
    fn local_defaults_to_ephemeral() {
        let config = Config::from_str(
            r#"{
            "networks": {
                "local": {
                    "bind": "localhost:8000"
                }
            }
        }"#,
        )
        .unwrap();

        let network = config.get_config().get_network("local").unwrap();
        if let ConfigNetwork::ConfigLocalProvider(local_network) = network {
            assert_eq!(local_network.r#type, NetworkType::Ephemeral);
        } else {
            panic!("not a local provider");
        }
    }

    #[test]
    fn local_can_override_to_persistent() {
        let config = Config::from_str(
            r#"{
            "networks": {
                "local": {
                    "bind": "localhost:8000",
                    "type": "persistent"
                }
            }
        }"#,
        )
        .unwrap();

        let network = config.get_config().get_network("local").unwrap();
        if let ConfigNetwork::ConfigLocalProvider(local_network) = network {
            assert_eq!(local_network.r#type, NetworkType::Persistent);
        } else {
            panic!("not a local provider");
        }
    }

    #[test]
    fn network_defaults_to_persistent() {
        let config = Config::from_str(
            r#"{
            "networks": {
                "somewhere": {
                    "providers": [ "https://1.2.3.4:5000" ]
                }
            }
        }"#,
        )
        .unwrap();

        let network = config.get_config().get_network("somewhere").unwrap();
        if let ConfigNetwork::ConfigNetworkProvider(network_provider) = network {
            assert_eq!(network_provider.r#type, NetworkType::Persistent);
        } else {
            panic!("not a network provider");
        }
    }

    #[test]
    fn network_can_override_to_ephemeral() {
        let config = Config::from_str(
            r#"{
            "networks": {
                "staging": {
                    "providers": [ "https://1.2.3.4:5000" ],
                    "type": "ephemeral"
                }
            }
        }"#,
        )
        .unwrap();

        let network = config.get_config().get_network("staging").unwrap();
        if let ConfigNetwork::ConfigNetworkProvider(network_provider) = network {
            assert_eq!(network_provider.r#type, NetworkType::Ephemeral);
        } else {
            panic!("not a network provider");
        }

        assert_eq!(
            config.get_config().get_network("staging").unwrap(),
            ConfigNetwork::ConfigNetworkProvider(ConfigNetworkProvider {
                providers: vec![String::from("https://1.2.3.4:5000")],
                r#type: NetworkType::Ephemeral,
            })
        );
    }

    #[test]
    fn get_correct_initialization_values() {
        let config = Config::from_str(
            r#"{
              "canisters": {
                "test_project": {
                  "initialization_values": {
                    "compute_allocation" : "100",
                    "memory_allocation": "8GB"
                  }
                }
              }
        }"#,
        )
        .unwrap();

        let config_interface = config.get_config();
        let compute_allocation = config_interface
            .get_compute_allocation("test_project")
            .unwrap()
            .unwrap();
        assert_eq!(100, compute_allocation);

        let memory_allocation = config_interface
            .get_memory_allocation("test_project")
            .unwrap()
            .unwrap();
        assert_eq!("8GB".parse::<Byte>().unwrap(), memory_allocation);

        let config_no_values = Config::from_str(
            r#"{
              "canisters": {
                "test_project_two": {
                }
              }
        }"#,
        )
        .unwrap();
        let config_interface = config_no_values.get_config();
        let compute_allocation = config_interface
            .get_compute_allocation("test_project_two")
            .unwrap();
        let memory_allocation = config_interface
            .get_memory_allocation("test_project_two")
            .unwrap();
        assert_eq!(None, compute_allocation);
        assert_eq!(None, memory_allocation);
    }
}