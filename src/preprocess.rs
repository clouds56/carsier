use crate::config::PackageConfig;
use crate::utils;
use crate::config::constant::*;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clap)]
pub struct Opts {
  #[clap(long)]
  pub features: Vec<String>,
  #[clap(long="entry-path")]
  pub start: Option<String>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Mod(Vec<String>, bool);
impl std::str::FromStr for Mod {
  type Err = &'static str;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    if s.is_empty() {
      return Err("empty string")
    }
    let mut mods = vec!["src".into()];
    let is_bin = match s {
      "bin" => true,
      "lib" => false,
      _ => {
        mods.extend(s.split('.').map(|s| s.to_string()));
        false
      }
    };
    Ok(Self(mods, is_bin))
  }
}
impl Mod {
  fn all_files(&self) -> Vec<PathBuf> {
    // TODO: iterator
    let path_str = self.0.join("/");
    if self.0.len() == 1 {
      vec![ (path_str + if self.1 {"/main.scala"} else { "/lib.scala" }).into() ]
    } else {
      vec![ (path_str.clone() + ".scala").into(), (path_str + "/lib.scala").into() ]
    }
  }

  pub fn files(&self) -> Vec<PathBuf> {
    let result: Vec<PathBuf> = self.all_files().into_iter().filter(|p| p.exists()).collect();
    info!("detect mod {:?}: {:?}", self.0.join("."), result.iter().map(|p| p.display()).collect::<Vec<_>>());
    result
  }
  pub fn is_empty(&self) -> bool { self.0.is_empty() }
  pub fn push(&mut self, s: String) { self.0.push(s) }
  pub fn concat(mut self, other: Mod) -> Self { self.0.extend(other.0); self }
  pub fn transform(mut self, prefix: &Self) -> Result<Self, failure::Error> {
    match self.0.first() {
      Some(s) if s == "%" => self.0[0] = "src".to_string(),
      Some(s) if s == "%%" => self.0 = prefix.0.clone().into_iter().chain(self.0.into_iter().skip(1)).collect(),
      Some(s) if s.starts_with("%") => return Err(failure::err_msg(format!("unknown percent {}", s))),
      _ => (),
    }
    Ok(self)
  }
  pub fn show(&self) -> String {
    self.0[1..].join(".")
  }
}

#[derive(Debug)]
struct Imports(Mod, Vec<Imports>);
impl Imports {
  pub fn new() -> Self {
    Imports(Mod(vec![], false), vec![])
  }
  pub fn from_str<S: Iterator<Item=char>>(s: &mut S, root: bool) -> Result<Vec<Imports>, failure::Error> {
    let mut result = Vec::new();
    let mut current = Self::new(); // a.b. {/**/} # except [, }] here
    let mut end_current = false;
    let mut ident = String::new();
    while let Some(c) = s.next() {
      match c {
        '%' if root && current.0.is_empty() => { ident.push(c) },
        '%' => return Err(failure::err_msg("symbols % in the middle")),
        ' ' | '\t' | '\n' | '\r' => (),
        ',' => {
          current.0.push(std::mem::replace(&mut ident, String::new()));
          result.push(std::mem::replace(&mut current, Self::new()));
          end_current = false;
        },
        '}' if !root => {
          current.0.push(std::mem::replace(&mut ident, String::new()));
          result.push(std::mem::replace(&mut current, Self::new()));
          break
        },
        '}' if root => return Err(failure::err_msg("trailing '}'")),
        _ if end_current => return Err(failure::err_msg("symbols after end_current")),
        '{' => {
          let still_root = root && current.0.is_empty();
          current.1 = Imports::from_str(s, still_root)?;
          end_current = true;
        },
        '.' => {
          current.0.push(std::mem::replace(&mut ident, String::new()))
        },
        _ => ident.push(c),
      }
    }
    if root {
      result.push(std::mem::replace(&mut current, Self::new()));
    }
    trace!("parse imports: {} {:?} {:?} {} {:?}", root, result, current, end_current, ident);
    Ok(result)
  }

  fn mods(&self) -> Vec<Mod> {
    let mut result = Vec::new();
    if self.1.is_empty() {
      return vec![self.0.clone()]
    }
    for i in &self.1 {
      trace!("for_mods {:?}",i);
      for j in i.mods() {
        result.push(self.0.clone().concat(j))
      }
      trace!("result: {:?}", result);
    }
    result
  }
}
impl std::str::FromStr for Imports {
  type Err = failure::Error;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let mut s = s.chars();
    let result = Imports::from_str(&mut s, true)?;
    trace!("parse Imports: {:?}", result);
    if s.count() == 0 {
      Ok(Imports(Mod(vec![], false), result))
    } else {
      Err(failure::err_msg("remain"))
    }
  }
}

fn preprocess(mods: &mut HashMap<PathBuf, String>, current: Mod) -> Result<(), failure::Error> {
  let mut queue = Vec::<Mod>::new();
  for path in current.files() {
    if let Some(content) = utils::load_content(&path)? {
      debug!("parsing: {}", path.display());
      mods.insert(path, current.show());
      for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") || line.starts_with("package") {
          continue;
        } else if line.starts_with("import") {
          debug!("parsing line: {}", line);
          let imports = line["import".len()..].trim().trim_end_matches(";");
          if imports.starts_with("%") {
            let new_mods = imports.parse::<Imports>()?.mods()
              .into_iter().map(|m| m.transform(&current)).collect::<Result<Vec<_>, _>>()?;
            debug!("found mod: {:?}", &new_mods);
            queue.extend(new_mods);
          }
        } else {
          break
        }
      }
    }
  }
  queue.into_iter().map(|c| preprocess(mods, c)).collect::<Result<(), _>>()?;
  Ok(())
}

fn detect_start() -> Mod {
  if Path::new("src/main.scala").exists() {
    "bin".parse().unwrap()
  } else {
    "lib".parse().unwrap()
  }
}

pub fn main(opts: Opts, config: &PackageConfig) -> Result<(), failure::Error> {
  let root = opts.start.map(|s| s.parse::<Mod>().map_err(failure::err_msg)).transpose()?.unwrap_or_else(detect_start);
  let mut mods = HashMap::new();
  preprocess(&mut mods, root)?;
  let mods_str = serde_json::to_string_pretty(&mods)?;
  std::fs::create_dir_all(target_dir().join("src"))?;
  let _ = utils::compare_and_write(target_dir().join("mods.json"), mods_str.as_bytes())?;
  Ok(())
}
