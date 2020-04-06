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

fn compile<P: AsRef<std::path::Path>>(path: P, cp: &str, files: &str) -> Result<std::path::PathBuf, failure::Error> {
  let mod_path = path.as_ref().ancestors().filter_map(|x| x.file_name()).collect::<Vec<_>>();
  let mut mod_name = mod_path.iter().skip(1).rev().skip(1).fold(std::ffi::OsString::new(), |mut acc, x| { acc.push(x); acc.push("."); acc });
  mod_name.push(path.as_ref().file_name().ok_or_else(|| failure::err_msg("no filename"))?);
  debug!("{:?} {:?}", mod_path, mod_name);
  let target = target_dir().join("build").join(mod_name).with_extension("jar");
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
  info!("compiled: {} => {}", path.as_ref().display(), target.display());
  Ok(target)
}

pub fn main(opts: Opts, config: &PackageConfig) -> Result<(), failure::Error> {
  resolve::main(opts.resolve, config)?;
  let path = "src/main.scala";
  preprocess::main(opts.preprocess, config)?;
  compile(path, "@target/deps.classpath", "@target/src_files")?;
  Ok(())
}
