#[macro_use] extern crate serde;

use std::path::Path;

pub mod config;
use config::{PackageConfig, repo::RepoConfig};

fn load_content<P: AsRef<Path>>(path: P) -> Result<String, failure::Error> {
  use std::io::Read;
  let mut f = std::fs::File::open(path)?;
  let mut content = String::new();
  f.read_to_string(&mut content)?;
  Ok(content)
}

fn load_repo_config<P: AsRef<Path>>(path: P) -> Result<RepoConfig, failure::Error> {
  let toml_str = load_content(path)?;
  let config: RepoConfig = toml::from_str(&toml_str)?;
  println!("{:#?}", config);
  Ok(config)
}

fn load_config<P: AsRef<Path>>(path: P) -> Result<PackageConfig, failure::Error> {
  let toml_str = load_content(path)?;
  let config: PackageConfig = toml::from_str(&toml_str)?;
  println!("{:#?}", config);
  Ok(config)
}

fn main() {
  load_repo_config("../configs/repo.toml").unwrap();
  load_config("Carsier.toml").expect("load config");
}
