use crate::config::PackageConfig;
use crate::utils;
use crate::config::constant::*;

use std::collections::{HashMap, BTreeMap};
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
    let mut mods = vec![];
    let is_bin = match s {
      "bin" => true,
      "lib" => false,
      _ => {
        mods = s.split('.').map(|s| s.to_string()).collect();
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
    if self.0.is_empty() {
      vec![ format!("src/{}.scala", if self.1 {"main"} else { "lib" }).into() ]
    } else {
      vec![ format!("src/{}.scala", path_str).into(), format!("src/{}/lib.scala", path_str).into() ]
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
  /// % means `crate`
  /// %% means `self`
  /// %^ means `super`
  /// %^^...^ means `super::super::...::super`
  pub fn transform(mut self, prefix: &Self) -> Result<Self, failure::Error> {
    match self.0.first() {
      Some(s) if s == "%" => self.0[0] = "src".to_string(),
      Some(s) if s == "%%" => self.0 = prefix.0.clone().into_iter().chain(self.0.into_iter().skip(1)).collect(),
      Some(s) if s.starts_with("%^") && s.trim_end_matches('^') == "%" => {
        let depth = s.len() - 1;
        if prefix.0.len() < depth {
          return Err(failure::err_msg(format!("parent {:?} depth out of range {}", prefix, depth)));
        }
        self.0 = prefix.0.iter().rev().skip(depth).rev().cloned().chain(self.0.into_iter().skip(1)).collect();
      },
      Some(s) if s.starts_with('%') => {
        return Err(failure::err_msg(format!("unknown percent {}", s)))
      },
      _ => (),
    }
    Ok(self)
  }
  pub fn show(&self) -> String {
    self.0.join(".")
  }
}

#[derive(Debug)]
struct Imports(Mod, Vec<Imports>);
// TODO support underscore and alias
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

  fn normalize(mut self, current: &Mod) -> Result<Self, failure::Error> {
    if self.0.is_empty() {
      self.1 = self.1.into_iter().map(|i| i.normalize(current)).collect::<Result<_, _>>()?
    } else {
      self.0 = self.0.transform(current)?;
    }
    Ok(self)
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

  fn display_inner(&self, mut root: bool) -> String {
    let mut s = self.0.show();
    if !self.0.is_empty() && !self.1.is_empty() {
      s.push('.');
    }
    if !self.0.is_empty() {
      root = false
    }
    if !self.1.is_empty() {
      if self.1.len() > 1 { s.push('{') }
      s += &self.1.iter().map(|i| i.display_inner(root)).collect::<Vec<_>>().join(", ");
      if self.1.len() > 1 { s.push('}') }
    }
    s
  }
  #[allow(dead_code)]
  fn display(&self, root: &str) -> String {
    format!("{}.{}", root, self.display_inner(true))
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

fn preprocess(mods: &mut HashMap<Mod, Vec<PathBuf>>, current: Mod, root_prefix: &str) -> Result<(), failure::Error> {
  use std::io::Write;
  let mut queue = Vec::<Mod>::new();
  for path in current.files() {
    let mut actual_current = current.clone();
    // let current = ();
    if let Some(content) = utils::load_content(&path)? {
      let out_path = target_dir().join(&path);
      std::fs::create_dir_all(out_path.parent().expect("parent"))?;
      let mut fout = std::fs::File::create(&out_path)?;
      info!("transform: {} => {}", path.display(), out_path.display());
      for line in content.lines() {
        let ln = line.trim();
        if ln.is_empty() || ln.starts_with("//") { }
        if ln.starts_with("package") {
          let package = ln["package".len()..].trim().trim_end_matches(';');
          if package.starts_with('%') {
            actual_current = package.parse::<Mod>().map_err(failure::err_msg)?.transform(&actual_current)?;
            debug!("current: {:?} => {:?}", current.show(), actual_current.show());
            let current_str = format!("{}{}{}", root_prefix, if actual_current.is_empty() { "" } else { "." }, actual_current.show());
            write!(fout, "package {};", current_str)?; // TODO _root_
            let mut sp = root_prefix.rsplitn(2, '.');
            let name = sp.next().unwrap();
            let root = sp.next().unwrap_or("_root_");
            write!(fout, "  import {}.{{{} => %}};", root, name)?;
            let mut sp = current_str.rsplitn(2, '.');
            let name = sp.next().unwrap();
            let root = sp.next().unwrap_or("_root_");
            writeln!(fout, "  import {}.{{{} => %%}};", root, name)?;
            continue;
          }
        } else if ln.starts_with("import") {
          debug!("parsing line: {}", ln);
          let imports = ln["import".len()..].trim().trim_end_matches(';');
          if imports.starts_with('%') {
            let imports = imports.parse::<Imports>()?.normalize(&actual_current)?;
            let new_mods = imports.mods();
            debug!("found mod: {:?}", &new_mods);
            queue.extend(new_mods);
            // writeln!(fout, "import {};", imports.display(root_prefix))?; // TODO _root_
            // continue;
          }
        }
        writeln!(fout, "{}", line)?;
      }
      mods.entry(actual_current).or_default().push(path.clone());
    }
  }
  for c in queue {
    if !mods.contains_key(&c) {
      preprocess(mods, c, root_prefix)?
    }
  }
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
  let root_prefix = format!("{}.{}", PACKAGE_PREFIX, config.package.name);
  let root = opts.start.map(|s| s.parse::<Mod>().map_err(failure::err_msg)).transpose()?.unwrap_or_else(detect_start);
  let mut mods = HashMap::new();
  preprocess(&mut mods, root, &root_prefix)?;
  let mods = mods.iter().map(|(i, v)| (i.show(), v)).collect::<BTreeMap<_,_>>();
  let mods_str = serde_json::to_string_pretty(&mods)?;
  let paths_str = mods.values().flat_map(|i| i.iter()).map(|i| target_dir().join(i).display().to_string()).collect::<Vec<_>>().join("\n");
  let _ = utils::compare_and_write(target_dir().join("mods.json"), mods_str.as_bytes())?;
  let _ = utils::compare_and_write(target_dir().join("src_files"), paths_str.as_bytes())?;
  Ok(())
}
