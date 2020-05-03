use crate::config::PackageConfig;
use crate::config::constant::*;
use crate::utils;

#[derive(Clap)]
pub struct Opts {
  #[clap(long, default_value = "coursier")]
  pub coursier: String,
}

fn dump_deps_in(config: &PackageConfig) -> Result<String, anyhow::Error> {
  let mut result = String::new();
  let edition = &config.package.edition;
  for (name, dep) in &config.dependencies {
    let dep = dep.as_dep();
    let dep = dep.as_ref();
    if let Some(org) = &dep.org {
      let name = if dep.java { name.to_string() } else { format!("{}_{}", name, edition) };
      let version = dep.version.as_coursier().ok_or_else(|| anyhow::Error::msg("cannot find a version"))?;
      result += &format!("{}:{}:{}\n", org, name, version);
    }
  }
  Ok(result)
}

pub fn main(opts: Opts, config: &PackageConfig) -> Result<(), anyhow::Error> {
  let coursier = &opts.coursier;
  let deps_in = dump_deps_in(config)?;
  let mut contd = utils::compare_and_write(target_dir().join("deps.in"), deps_in.as_bytes())?;
  let deps_out = contd.exists_and_write(target_dir().join("deps.out"), || {
    utils::call(coursier, vec!["resolve", "--quiet"].into_iter().chain(deps_in.lines())).map(|s| s.into())
  })?;
  let deps_out = String::from_utf8(deps_out)?;
  contd.exists_and_write(target_dir().join("deps.classpath"), || {
    utils::call(coursier, vec!["fetch", "--quiet", "--classpath"].into_iter().chain(deps_out.lines()))
      .map(|s| format!("{:?}", s.trim_end_matches('\n')).into())
  })?;
  Ok(())
}
