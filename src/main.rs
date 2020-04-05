#[macro_use] extern crate log;
#[macro_use] extern crate failure;
#[macro_use] extern crate serde;
#[macro_use] extern crate clap;
use clap::Clap;

use std::path::{Path, PathBuf};

pub mod config;
mod init;
mod utils;

use config::{PackageConfig, repo::RepoConfig};

#[derive(Clap)]
pub struct Opts {
  #[clap(short = "C", default_value = ".")]
  workdir: PathBuf,
  #[clap(long = "config")]
  config: Option<PathBuf>,
  #[clap(subcommand)]
  subcmd: SubCommand,
}

#[derive(Clap)]
pub enum SubCommand {
  New(init::NewOpts),
  Init(init::Opts),
  // TODO: https://github.com/clap-rs/clap/issues/1672
  // #[clap(external_subcommand)]
  // External(Vec<String>),
  External,
}

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

fn init_logger(path: Option<&Path>) {
  use simplelog::*;
  let mut loggers: Vec<Box<(dyn SharedLogger)>> =  vec![ TermLogger::new(LevelFilter::Info, Config::default(), TerminalMode::Mixed).unwrap(), ];
  if let Some(path) = path {
    if let Ok(file) = std::fs::File::create(path) {
      loggers.push(WriteLogger::new(LevelFilter::Info, Config::default(), file))
    }
  }
  CombinedLogger::init(loggers).unwrap();
}

fn main() {
  let opts: Opts = Opts::parse();
  debug!("workdir {:?}", opts.workdir.display());
  std::env::set_current_dir(&opts.workdir).expect("chdir failed");
  let subcmd = match opts.subcmd {
    SubCommand::Init(sub_opts) => {
      init_logger(None);
      init::main(sub_opts).expect("execute failed");
      return
    },
    SubCommand::New(sub_opts) => {
      init_logger(None);
      std::fs::create_dir(&sub_opts.foldername).expect("folder already exists");
      std::env::set_current_dir(&sub_opts.foldername).expect("chdir failed");
      init::main(sub_opts.opts).expect("execute failed");
      return
    },
    subcmd => subcmd,
  };
  // load_repo_config("../configs/repo.toml").unwrap();
  let config = load_config(opts.config.unwrap_or_else(|| config::constant::toml_name().into())).expect("load config");
}
