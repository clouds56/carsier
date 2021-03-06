use std::collections::BTreeMap;

pub mod constant;
pub mod repo;
mod version;
pub use version::VersionRange;

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
  pub package: Package,
  pub dependencies: BTreeMap<String, DependencyLike>,
  #[serde(default)]
  pub features: BTreeMap<String, FeatureLike>,
  #[serde(default)]
  pub resources: Vec<Resource>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Package {
  pub name: String,
  pub version: Version,
  #[serde(default)]
  pub authors: Vec<String>,
  pub edition: String,
  #[serde(default = "constant::default_registry")]
  pub registry: String,
  #[serde(flatten)]
  pub others: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencyLike {
  Version(VersionRange),
  Full(Dependency),
}
impl DependencyLike {
  pub fn as_dep(&self) -> std::borrow::Cow<'_, Dependency> {
    use std::borrow::Cow;
    match self {
      DependencyLike::Full(dep) => Cow::Borrowed(dep),
      DependencyLike::Version(version) => Cow::Owned(version.clone().into()),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
  pub version: VersionRange,
  #[serde(default)]
  pub features: Vec<String>,
  #[serde(default)]
  pub java: bool,
  pub org: Option<String>,
  #[serde(flatten)]
  pub others: BTreeMap<String, String>,
}
impl From<VersionRange> for Dependency {
  fn from(s: VersionRange) -> Self {
    Self {
      version: s,
      features: Default::default(),
      java: Default::default(),
      org: Default::default(),
      others: Default::default(),
    }
  }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FeatureLike {
  Group(Vec<String>),
  Full(Feature),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
  #[serde(default)]
  pub conflict: bool,
  #[serde(rename = "virtual", default)]
  pub computed: bool,
  pub group: Vec<String>,
}

pub type Version = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
  pub include: String,
}
