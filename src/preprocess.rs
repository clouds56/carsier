use crate::config::PackageConfig;
use crate::utils;
use crate::config::constant::*;
use failure::ResultExt;

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
  Relative(usize), Absolute, Root(String),
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Mod(Prefix, Vec<String>);

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

  pub fn from_path(path: &Path, root: &Path) -> Self {
    let common_count = path.components().zip(root.components()).take_while(|(a, b)| a == b).count();
    let mut modpath = path.components().skip(common_count).map(|s| s.as_os_str().to_string_lossy().to_string()).collect::<Vec<_>>();
    if let Some(mut filename) = modpath.pop() {
      if filename == "lib.scala" {}
      if filename == "main.scala" && modpath.is_empty() {}
      else if filename.ends_with(".scala") {
        filename.truncate(filename.len() - ".scala".len());
        modpath.push(filename)
      } else {
        modpath.push(filename)
      }
    }
    let rel = root.components().count() - common_count;
    Self(if rel == 0 { Prefix::Absolute } else { Prefix::Relative(rel) }, modpath)
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
            Prefix::Root(_) | Prefix::Absolute => return Err("mod transform out of absolute"),
            Prefix::Relative(n2) => Prefix::Relative(n2 + n - new_path.len()),
          };
          Self(prefix, self.1)
        }
      }
    })
  }

  fn show(&self) -> String {
    self.1.join(".")
  }

  fn is_empty(&self) -> bool {
    self.1.is_empty()
  }
}
impl std::str::FromStr for Mod {
  type Err = &'static str;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let mut split = s.split(".");
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

fn preprocess(mods: &mut BTreeMap<Mod, Vec<PathBuf>>, pattern: &str, root: &Path, crate_name: &str) -> Result<(), failure::Error> {
  use std::io::Write;
  for path in glob::glob(pattern).context("pattern not valid")?.filter_map(|i| i.ok()) {
    let current = Mod::from_path(&path, root);
    // let current = ();
    if let Some(content) = utils::load_content(&path)? {
      let out_path = target_dir().join(&path);
      std::fs::create_dir_all(out_path.parent().expect("parent"))?;
      let mut fout = std::fs::File::create(&out_path)?;
      info!("transform: {} => {}", path.display(), out_path.display());
      let mut multicomments = false;
      let mut processed = false;
      for line in content.lines() {
        let ln = line.trim();
        if processed {}
        else if multicomments {
          if ln.ends_with("*/") {
            multicomments = false
          }
        } else if ln.is_empty() || ln.starts_with("//") {
          ()
        } else if ln.starts_with("/*") {
          if ln.ends_with("*/") && ln.len() > 3 {
            multicomments = true
          }
        } else if ln.starts_with("package") {
          let package = ln["package".len()..].trim().trim_end_matches(';');
          if package.starts_with('%') {
            let actual_current = package.parse::<Mod>().map_err(failure::err_msg)?.transform(&current).map_err(failure::err_msg)?;
            debug!("current: {:?} => {:?}", current.show(), actual_current.show());
            let current_str = format!("{}.{}{}{}", PACKAGE_PREFIX, crate_name, if actual_current.is_empty() { "" } else { "." }, actual_current.show());
            write!(fout, "package {};", current_str)?; // TODO _root_
            write!(fout, " import {}.{{{} => %}};", PACKAGE_PREFIX, crate_name)?;
            let mut sp = current_str.rsplitn(2, '.');
            let name = sp.next().unwrap();
            let root = sp.next().unwrap_or("_root_");
            writeln!(fout, " import {}.{{{} => %%}};", root, name)?;
            mods.entry(actual_current).or_default().push(path.clone());
            processed = true;
            continue
          }
        }
        writeln!(fout, "{}", line)?;
      }
    }
  }
  Ok(())
}

pub fn main(opts: Opts, config: &PackageConfig) -> Result<(), failure::Error> {
  let src_root = opts.src_root.clone().unwrap_or_else(|| opts.include.split('/').take_while(|s| !s.contains('*')).collect::<Vec<_>>().join("/"));
  let mut mods = BTreeMap::new();
  preprocess(&mut mods, &opts.include, src_root.as_ref(), &config.package.name)?;
  let mods = mods.iter().map(|(i, v)| (i.show(), v)).collect::<BTreeMap<_,_>>();
  let mods_str = serde_json::to_string_pretty(&mods)?;
  let paths_str = mods.values().flat_map(|i| i.iter()).map(|i| target_dir().join(i).display().to_string()).collect::<Vec<_>>().join("\n");
  let _ = utils::compare_and_write(target_dir().join("mods.json"), mods_str.as_bytes())?;
  let _ = utils::compare_and_write(target_dir().join("src_files"), paths_str.as_bytes())?;
  Ok(())
}
