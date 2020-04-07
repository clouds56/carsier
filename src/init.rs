use crate::config::constant::*;
use crate::utils;
use std::fs::*;
use std::io::Write;

#[derive(Clap)]
pub struct Opts {
  #[clap(long = "name")]
  pub name: Option<String>,
}

#[derive(Clap)]
pub struct NewOpts {
  pub foldername: String,
  #[clap(flatten)]
  pub opts: Opts,
}

const TOML_TEMPLATE: &str = r#"
[package]
name = "<name>"
version = "<version>"
authors = ["<author>"]
edition = "<edition>"

[dependencies]
"#;

const INGORE_CONTENT: &str = r#"
/target
.metals
"#;

const HELLO_CODE: &str = r#"
package %%;

object Main extends App {
  println("hello, world!");
}
"#;

fn get_author_from_git() -> Option<String> {
  let username = utils::call("git", &["config", "user.name"]).ok()?;
  let email = utils::call("git", &["config", "user.email"]).ok()?;
  format!("{} <{}>", username.trim(), email.trim()).into()
}

fn init_git() -> Result<(), failure::Error> {
  if utils::call("git", &["rev-parse"]).is_err() {
    utils::call("git", &["init"])?;
  }
  Ok(())
}

pub fn build_template(name: &str, edition: &str) -> String {
  TOML_TEMPLATE.trim_start()
    .replace("<name>", name)
    .replace("<version>", "0.1.0")
    .replace("<author>", &get_author_from_git().unwrap_or_else(default_author))
    .replace("<edition>", edition)
}

pub fn main(opts: Opts) -> Result<(), failure::Error> {
  let name = match opts.name {
    Some(name) => name,
    None => std::env::current_dir()?.file_name().ok_or_else(|| failure::err_msg("current dir is root"))?.to_string_lossy().to_string(),
  };
  info!("init project {}", name);
  let mut toml_file = OpenOptions::new().write(true).create_new(true).open(toml_name())?;
  toml_file.write_all(build_template(&name, &scala_edition()).as_bytes())?;

  init_git().ok();
  if let Ok(mut ignore_file) = OpenOptions::new().write(true).create_new(true).open(".gitignore") {
    ignore_file.write_all(INGORE_CONTENT.trim_start().as_bytes())?;
  }
  create_dir_all("src").ok();
  if let Ok(mut src_file) = OpenOptions::new().write(true).create_new(true).open("src/main.scala") {
    src_file.write_all(HELLO_CODE.trim_start().as_bytes())?;
  }
  Ok(())
}
