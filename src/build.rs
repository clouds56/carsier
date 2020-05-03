#![allow(dead_code)]

use std::rc::Rc;
use std::collections::BTreeMap;
use std::path::Path;
use failure::ResultExt;
use crate::config::PackageConfig;
use crate::{resolve, preprocess};
use crate::utils;
use crate::config::constant::*;

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
  name: TargetName,
  profile: Profile,
  features: BTreeMap<String, Rc<Feature>>,
}

fn get_target(opts: &Opts, _config: &PackageConfig) -> Result<Vec<Target>, failure::Error> {
  let mut names = Vec::new();
  if Path::new("src/lib.scala").exists() {
    names.push(TargetName::Lib)
  }
  if Path::new("src/main.scala").exists() {
    names.push(TargetName::BinMain)
  }
  if names.is_empty() {
    return Err(failure::err_msg("no target found"))
  }
  let targets = names.into_iter().map(|name| Target {
    name,
    profile: if opts.target.release { Profile::Release } else { Profile::Debug },
    features: BTreeMap::new()
  }).collect();
  Ok(targets)
}

fn compile(target: Target, cp: &str, files: &str) -> Result<std::path::PathBuf, failure::Error> {
  let target_name = target.name.to_string();
  let target = target_dir().join("build").join(&target_name).with_extension("jar");
  std::fs::create_dir_all(target.parent().unwrap())?;
  let opts = vec![
    "--class-path", cp,
    "--source-path", "src",
  ];
  utils::call("scalac", opts.into_iter().map(std::ffi::OsStr::new).chain(vec![
    // "--dependency-file".as_ref(), target_dir().join("scala_dep").as_ref(),
    files.as_ref(),
    "-d".as_ref(), target.as_os_str(),
  ].into_iter()))?;
  info!("compiled: {} => {}", files, target_name);
  Ok(target)
}

pub fn main(opts: Opts, config: &PackageConfig) -> Result<(), failure::Error> {
  let targets = get_target(&opts, &config).context("parse target failed")?;
  resolve::main(opts.resolve, config).context("resolve failed")?;
  preprocess::main(opts.preprocess, config).context("preprocess failed")?;
  for target in targets {
    compile(target, "@target/deps.classpath", "@target/src_files")?;
  }
  Ok(())
}
