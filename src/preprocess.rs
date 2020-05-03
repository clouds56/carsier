use std::collections::BTreeSet;
use crate::config::PackageConfig;
use crate::utils;
use crate::config::constant::*;
use crate::build::Target;
use anyhow::Context;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clap)]
pub struct Opts {
  #[clap(long)]
  pub features: Vec<String>,
  #[clap(long="include", default_value="src/**/*.scala")]
  pub include: String,
  #[clap(long="src-root")]
  pub src_root: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Prefix {
  Relative(usize), Absolute, Root(String), EntryPoint(String),
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Mod(Prefix, Vec<String>);

#[derive(Debug, Serialize, Deserialize)]
pub struct Unit {
  path: PathBuf,
  features: BTreeSet<String>,
}

impl Mod {
  // fn src_file(s: &str) -> PathBuf {
  //   format!("src/{}.scala", s).into()
  // }
  // fn src_lib_file(s: &str) -> PathBuf {
  //   format!("src/{}/lib.scala", s).into()
  // }
  // fn all_files(&self) -> Vec<PathBuf> {
  //   // TODO: iterator
  //   let path_str = self.1.join("/");
  //   if self.1.is_empty() {
  //     vec![ Mod::src_file("main"), Mod::src_file("lib") ]
  //   } else {
  //     vec![ Mod::src_file(&path_str), Mod::src_lib_file(&path_str) ]
  //   }
  // }

  // pub fn files(&self) -> Vec<PathBuf> {
  //   let result: Vec<PathBuf> = self.all_files().into_iter().filter(|p| p.exists()).collect();
  //   info!("detect mod {:?}: {:?}", self.1.join("."), result.iter().map(|p| p.display()).collect::<Vec<_>>());
  //   result
  // }

  pub fn from_path(path: &Path, root: &Path) -> (Self, BTreeSet<String>) {
    let mut features = BTreeSet::new();
    let common_count = path.components().zip(root.components()).take_while(|(a, b)| a == b).count();
    let mut modpath = path.components().skip(common_count).map(|s| s.as_os_str().to_string_lossy().to_string()).collect::<Vec<_>>();
    let mut prefix = match root.components().count() - common_count {
      0 => Prefix::Absolute,
      n => Prefix::Relative(n),
    };
    if let Some(mut filename) = modpath.pop() {
      if filename.ends_with(".scala") {
        filename.truncate(filename.len() - ".scala".len());
      }
      let mut sp = filename.split('-').map(|s| s.to_string());
      let filename = sp.next().unwrap();
      features.append(&mut sp.collect());
      if (filename == "lib" || filename == "main") && modpath.is_empty() {
        prefix = Prefix::EntryPoint(filename.to_string())
      } else if filename != "lib" {
        modpath.push(filename)
      }
    }
    (Self(prefix, modpath), features)
  }

  pub fn transform(mut self, base: &Self) -> Result<Self, &'static str> {
    Ok(match self.0 {
      Prefix::Root(_) | Prefix::Absolute => self,
      Prefix::Relative(n) => {
        let mut new_path = base.1.clone();
        if new_path.len() >= n {
          new_path.truncate(new_path.len() - n);
          new_path.append(&mut self.1);
          Self(base.0.clone(), new_path)
        } else {
          let prefix = match base.0 {
            // fixme: is cross root supported?
            Prefix::Root(_) | Prefix::Absolute | Prefix::EntryPoint(_) => return Err("mod transform out of absolute"),
            Prefix::Relative(n2) => Prefix::Relative(n2 + n - new_path.len()),
          };
          Self(prefix, self.1)
        }
      }

      Prefix::EntryPoint(_) => unreachable!("from str should never be entrypoint")
    })
  }

  fn path(&self) -> String {
    self.1.join(".")
  }
  fn show(&self) -> String {
    format!("{}.{}", self.0.to_string(), self.1.join("."))
  }

  fn is_empty(&self) -> bool {
    self.1.is_empty()
  }
}
impl std::str::FromStr for Mod {
  type Err = &'static str;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let mut split = s.split('.');
    let prefix_str = split.next().unwrap();
    let prefix = match prefix_str {
      "%" => Prefix::Absolute,
      "%%" => Prefix::Relative(0),
      s if s.starts_with("%^") && s.trim_end_matches('^') == "%" => {
        Prefix::Relative(s.len() - 1)
      },
      s if !s.starts_with('%') => Prefix::Root(s.to_string()),
      _ => return Err("unknown %")
    };
    Ok(Self(prefix, split.map(|i| i.to_string()).collect()))
  }
}
impl ToString for Prefix {
  fn to_string(&self) -> String {
    match self {
      Prefix::EntryPoint(s) => format!("@{}", s),
      Prefix::Absolute => "%".to_string(),
      Prefix::Relative(0) => "%%".to_string(),
      Prefix::Relative(n) => format!("%{}", "^".repeat(*n)),
      Prefix::Root(s) => s.clone(),
    }
  }
}

fn preprocess(mods: &mut BTreeMap<Mod, Vec<Unit>>, pattern: &str, root: &Path, crate_name: &str, registry_name: &str) -> Result<(), anyhow::Error> {
  use std::io::Write;
  for path in glob::glob(pattern).context("pattern not valid")?.filter_map(|i| i.ok()) {
    let (current, features) = Mod::from_path(&path, root);
    // let current = ();
    if let Some(content) = utils::load_content(&path)? {
      let out_path = target_dir().join(&path);
      std::fs::create_dir_all(out_path.parent().expect("parent"))?;
      let mut fout = std::fs::File::create(&out_path)?;
      info!("transform: {} => {}", path.display(), out_path.display());
      let mut multicomments = false;
      let mut actual_current = None;
      for line in content.lines() {
        let ln = line.trim();
        if actual_current.is_some() {}
        else if multicomments {
          if ln.ends_with("*/") {
            multicomments = false
          }
        } else if ln.is_empty() || ln.starts_with("//") {
        } else if ln.starts_with("/*") {
          if ln.ends_with("*/") && ln.len() > 3 {
            multicomments = true
          }
        } else if ln.starts_with("package") {
          let package = ln["package".len()..].trim().trim_end_matches(';');
          actual_current = if package.starts_with('%') {
            let actual_current = package.parse::<Mod>().map_err(anyhow::Error::msg)?.transform(&current).map_err(anyhow::Error::msg)?;
            debug!("current: {:?} => {:?}", current.show(), actual_current.show());
            let current_str = format!("{}.{}{}{}", registry_name, crate_name, if actual_current.is_empty() { "" } else { "." }, actual_current.path());
            write!(fout, "package {};", current_str)?; // TODO _root_
            // write!(fout, " import _root_.{{{} => %:}};", PACKAGE_PREFIX)?;
            write!(fout, " import {}.{{{} => %}};", registry_name, crate_name)?;

            let len = actual_current.1.len();
            write!(fout, "import {}.{{{} => {}}};", registry_name, crate_name, Prefix::Relative(len).to_string())?;
            for i in 0..len {
              write!(fout, "import %{}{}.{{{} => {}}};",
                if i == 0 {""} else {"."},
                actual_current.1[..i].join("."),
                actual_current.1[i],
                Prefix::Relative(len-i-1).to_string())?;
            }
            writeln!(fout, "")?;
            Some(actual_current)
          } else {
            writeln!(fout, "{}", line)?;
            Some(package.parse::<Mod>().map_err(anyhow::Error::msg)?)
          };
          continue
        }
        writeln!(fout, "{}", line)?;
      }
      if let Some(current) = actual_current {
        mods.entry(current).or_default().push(Unit{ path, features });
      }
    }
  }
  Ok(())
}

pub fn main(opts: Opts, config: &PackageConfig) -> Result<(), anyhow::Error> {
  let src_root = opts.src_root.clone().unwrap_or_else(|| opts.include.split('/').take_while(|s| !s.contains('*')).collect::<Vec<_>>().join("/"));
  let mut mods = BTreeMap::new();
  preprocess(&mut mods, &opts.include, src_root.as_ref(), &config.package.name, &config.package.registry)?;
  let mods = mods.iter().map(|(i, v)| (i.show(), v)).collect::<BTreeMap<_,_>>();
  let mods_str = serde_json::to_string_pretty(&mods)?;
  let _ = utils::compare_and_write(target_dir().join("mods.json"), mods_str.as_bytes())?;
  Ok(())
}

pub fn src_files(target: &Target, units: &BTreeMap<String, Vec<Unit>>) -> Result<String, anyhow::Error> {
  let base = target.name.to_string();
  let features = target.features.keys().cloned().collect::<BTreeSet<_>>();
  let features_str = format!("{}{}", base, features.iter().map(|f| format!("-{}", f)).collect::<Vec<_>>().join(""));
  let paths_str = units.iter().filter(|(s, _)| !s.starts_with('@'))
    .flat_map(|(_, i)| i.iter()).filter(|i| i.features.is_empty() || !i.features.is_disjoint(&features))
    .chain(units.get(&format!("@{}.", base)).ok_or_else(|| anyhow::Error::msg("entrypoint not found"))?.iter())
    .map(|i| target_dir().join(&i.path).display().to_string()).collect::<Vec<_>>().join("\n");
  let _ = utils::compare_and_write(target_dir().join("src_files").join(&features_str), paths_str.as_bytes())?;
  Ok(features_str)
}
