#[macro_use] extern crate log;
#[macro_use] extern crate failure;
#[macro_use] extern crate serde;
#[macro_use] extern crate clap;
extern crate semver_parser as semver;
use clap::Clap;

use std::path::{Path, PathBuf};

pub mod config;
mod init;
mod utils;
mod resolve;
mod preprocess;
mod build;

use config::{PackageConfig, repo::RepoConfig};

#[derive(Clap)]
pub struct Opts {
  #[clap(short = "C", default_value = ".")]
  workdir: PathBuf,
  #[clap(long = "config")]
  config: Option<PathBuf>,
  #[clap(short = "v", long)]
  verbose: bool,
  #[clap(subcommand)]
  subcmd: SubCommand,
}

#[derive(Clap)]
pub struct ExternelOpts {
  args: Vec<String>
}

#[derive(Clap)]
pub enum SubCommand {
  New(init::NewOpts),
  Init(init::Opts),
  Build(build::Opts),
  Resolve(resolve::Opts),
  // TODO: https://github.com/clap-rs/clap/issues/1672
  // #[clap(external_subcommand)]
  // External(Vec<String>),
  #[clap(name = "-")]
  External(ExternelOpts),
}

pub fn load_repo_config<P: AsRef<Path>>(path: P) -> Result<RepoConfig, failure::Error> {
  let toml_str = utils::load_content(path)?.ok_or_else(|| failure::err_msg("open repo_config file"))?;
  let config: RepoConfig = toml::from_str(&toml_str)?;
  dbg!(&config);
  Ok(config)
}

fn load_config<P: AsRef<Path>>(path: P) -> Result<PackageConfig, failure::Error> {
  let toml_str = utils::load_content(path)?.ok_or_else(|| failure::err_msg("open config file"))?;
  let config: PackageConfig = toml::from_str(&toml_str)?;
  dbg!(&config);
  Ok(config)
}

fn init_logger(verbose: bool, path: Option<&Path>) {
  use simplelog::*;
  let level = if verbose { LevelFilter::Debug } else { LevelFilter::Info };
  let mut loggers: Vec<Box<(dyn SharedLogger)>> =  vec![ TermLogger::new(level, Config::default(), TerminalMode::Mixed).unwrap(), ];
  if let Some(path) = path {
    if let Ok(file) = std::fs::File::create(path) {
      loggers.push(WriteLogger::new(LevelFilter::Info, Config::default(), file))
    }
  }
  CombinedLogger::init(loggers).unwrap();
}

fn main() {
  let opts: Opts = Opts::parse();
  let verbose = opts.verbose;
  debug!("workdir {:?}", opts.workdir.display());
  std::env::set_current_dir(&opts.workdir).expect("chdir failed");
  let subcmd = match opts.subcmd {
    SubCommand::Init(sub_opts) => {
      init_logger(verbose, None);
      init::main(sub_opts).expect("execute failed");
      return
    },
    SubCommand::New(sub_opts) => {
      init_logger(verbose, None);
      std::fs::create_dir(&sub_opts.foldername).expect("folder already exists");
      std::env::set_current_dir(&sub_opts.foldername).expect("chdir failed");
      init::main(sub_opts.opts).expect("execute failed");
      return
    },
    SubCommand::External(ExternelOpts { args }) => {
      let cmd = format!("{}-{}", config::constant::NAME, args.first().expect("expect external command"));
      utils::call(&cmd, &args[1..]).unwrap_or_else(|e| panic!("external sub-command {} failed: {:?}", cmd, e));
      return
    },
    subcmd => subcmd,
  };
  // load_repo_config("../configs/repo.toml").unwrap();
  let config = load_config(opts.config.unwrap_or_else(|| config::constant::toml_name().into())).expect("load config");
  std::fs::create_dir_all("target").expect("create target dir");
  match subcmd {
    SubCommand::Init(_) | SubCommand::New(_) | SubCommand::External(_) => unreachable!("already handled"),
    SubCommand::Resolve(opts) => {
      init_logger(verbose, Some("target/resolve.log".as_ref()));
      resolve::main(opts, &config).unwrap();
    },
    SubCommand::Build(opts) => {
      init_logger(verbose, Some("target/build.log".as_ref()));
      build::main(opts, &config).unwrap();
    }
  }
}
