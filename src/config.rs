use std::collections::HashMap;

pub mod repo;

/// a config file looks like
/// ```
/// [package]
/// name = "demo"
/// version = "0.1.0"
/// authors = ["Clouds Flowing <clouds.flowing@gmail.com>"]
/// edition = "2.13"
///
/// [dependencies]
/// breeze = { version = "*", binary = "maven2" }
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct PackageConfig {
  package: Package,
  dependencies: HashMap<String, DependencyLike>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Package {
  name: String,
  version: Version,
  #[serde(default)]
  authors: Vec<String>,
  edition: String,
  #[serde(flatten)]
  others: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencyLike {
  String(Version),
  Full(Dependency),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
  version: Version,
  #[serde(default)]
  features: Vec<String>,
  #[serde(flatten)]
  others: HashMap<String, String>,
}

// TODO support ~0.3 ^3.1.2 =0.1.0
pub type Version = String;
