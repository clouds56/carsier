use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RepoConfig {
  #[serde(default)]
  repos: HashMap<String, RepoLike>
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RepoLike {
  Url(String),
  Full(Repo),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Repo {
  url: String,
}
