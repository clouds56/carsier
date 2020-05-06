#![allow(dead_code)]

use std::rc::Rc;
use std::collections::BTreeMap;
use std::path::Path;
use anyhow::Context;
use crate::{resolve, preprocess};
use crate::utils;
use crate::config::{PackageConfig, Resource, constant::*};

#[derive(Clap)]
pub struct Opts {
  #[clap(flatten)]
  pub target: TargetOpts,
  #[clap(flatten)]
  pub preprocess: preprocess::Opts,
  #[clap(flatten)]
  pub resolve: resolve::Opts,
}

#[derive(Clap)]
pub struct TargetOpts {
  #[clap(long)]
  pub release: bool,
}

#[derive(Debug, Clone)]
pub enum TargetName {
  Lib, BinMain, Bin(String), Example(String), Test(String),
}
impl ToString for TargetName {
  fn to_string(&self) -> String {
    match self {
      TargetName::Lib => "lib".to_string(),
      TargetName::BinMain => "main".to_string(),
      TargetName::Bin(s) => format!("bin_{}", s),
      TargetName::Example(s) => format!("example_{}", s),
      TargetName::Test(s) => format!("test_{}", s),
    }
  }
}
#[derive(Debug, Clone, Copy)]
pub enum Profile {
  Debug, Release, RelWithDebugInfo, Test
}

#[derive(Debug, Clone, Copy)]
pub enum FeatureFlag {
  /// features in group could only select one
  Conflict,
  /// features in group
  Virtual,
  /// features would set a group of package
  Set,
  /// features indicate an package with same name
  Package,
}
#[derive(Debug, Clone)]
pub struct Feature {
  name: String,
  group: Vec<Rc<Feature>>,
  flag: FeatureFlag,
}

pub struct Target {
  pub name: TargetName,
  pub profile: Profile,
  pub features: BTreeMap<String, Rc<Feature>>,
}

fn get_target(opts: &Opts, _config: &PackageConfig) -> Result<Vec<Target>, anyhow::Error> {
  let mut names = Vec::new();
  if Path::new("src/lib.scala").exists() {
    names.push(TargetName::Lib)
  }
  if Path::new("src/main.scala").exists() {
    names.push(TargetName::BinMain)
  }
  if names.is_empty() {
    return Err(anyhow::Error::msg("no target found"))
  }
  let targets = names.into_iter().map(|name| Target {
    name,
    profile: if opts.target.release { Profile::Release } else { Profile::Debug },
    features: BTreeMap::new()
  }).collect();
  Ok(targets)
}

fn compile(target: Target, cp: &str, files: &str) -> Result<std::path::PathBuf, anyhow::Error> {
  let target_name = target.name.to_string();
  let target = target_dir().join("build").join(&target_name).with_extension("jar");
  std::fs::create_dir_all(target.parent().unwrap())?;
  let opts = vec![
    "--class-path", cp,
    "--source-path", "src",
    "@target/plugin_opts"
  ];
  utils::call("scalac", opts.into_iter().map(std::ffi::OsStr::new).chain(vec![
    // "--dependency-file".as_ref(), target_dir().join("scala_dep").as_ref(),
    files.as_ref(),
    "-d".as_ref(), target.as_os_str(),
  ].into_iter()))?;
  info!("compiled: {} => {}", files, target_name);
  Ok(target)
}

fn package(target: &Path, resources: &Vec<Resource>) -> Result<(), anyhow::Error> {
  let resource_files = resources.iter().map(|r| glob::glob(&r.include)).collect::<Result<Vec<_>, _>>()?
    .into_iter().flat_map(|g| g.into_iter()).filter_map(|i| i.ok()).collect::<Vec<_>>();
  if resource_files.is_empty() {
    return Ok(())
  }
  let resource_str = resource_files.iter().map(|f| f.display().to_string()).collect::<Vec<_>>().join("\n");
  let _ = utils::compare_and_write(target_dir().join("resources.txt"), resource_str.as_bytes())?;
  utils::call("jar", vec!["--update".as_ref(), "--file".as_ref(), target.as_os_str(), "@target/resources.txt".as_ref()].into_iter())?;
  Ok(())
}

pub fn main(opts: Opts, config: &PackageConfig) -> Result<(), anyhow::Error> {
  let targets = get_target(&opts, &config).context("parse target failed")?;
  resolve::main(opts.resolve, config).context("resolve failed")?;
  preprocess::main(opts.preprocess, config).context("preprocess failed")?;
  ensure_plugin().context("write plugin failed")?;
  let _ = utils::compare_and_write(target_dir().join("plugin_opts"), format!("-Xplugin:target/plugin.jar -P:moduler:name={}", config.package.name).as_bytes())?;
  let units: BTreeMap<String, Vec<preprocess::Unit>> = serde_json::from_reader(std::fs::File::open("target/mods.json").context("open mods.json")?).context("read mods.json")?;
  for target in targets {
    let units_file = preprocess::src_files(&target, &units, false).context("gen src_files")?;
    let result = compile(target, "@target/deps.classpath", &format!("@target/src_files/{}", units_file))?;
    package(&result, &config.resources)?;
  }
  Ok(())
}
