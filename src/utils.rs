use std::path::{Path, PathBuf};

#[derive(Debug, Fail)]
#[fail(display = "call process {} failed with code {}", 0, 1)]
pub struct CallError(pub String, pub i32, pub std::process::Output);
impl CallError {
  fn new<S: AsRef<str>>(s: S, i: std::process::Output) -> Self {
    Self(s.as_ref().to_string(), i.status.code().unwrap_or(-1), i)
  }
}

pub fn call<Args, S1>(cmd: &str, args: Args) -> Result<String, failure::Error>
  where Args: IntoIterator<Item = S1>, S1: AsRef<std::ffi::OsStr> {
  use std::process::*;
  // TODO: encoding
  let args = args.into_iter().collect::<Vec<_>>();
  debug!("call: {} {:?}", cmd, args.iter().map(|i| i.as_ref()).collect::<Vec<_>>());
  let p = Command::new(cmd).stdin(Stdio::null()).args(args).output()?;

  if p.status.success() {
    Ok(String::from_utf8(p.stdout)?)
  } else {
    Err(CallError::new(cmd, p))?
  }
}

trait PathExt {
  fn lock(&self) -> Option<PathBuf>;
}
impl PathExt for Path {
  fn lock(&self) -> Option<PathBuf> {
    let mut filename = self.file_name()?.to_os_string();
    filename.push(".lock");
    self.with_file_name(filename).into()
  }
}
#[must_use]
#[derive(Debug, Clone, Copy)]
pub enum FileDep {
  Unchanged,
  Touched,
}
#[allow(dead_code)]
impl FileDep {
  pub fn check(self, check: bool) -> Self {
    if check { self } else { FileDep::Touched }
  }
  pub fn exists<P: AsRef<Path>>(self, path: P) -> Self {
    self.check(path.as_ref().exists())
  }
  pub fn exists_and_write<P: AsRef<Path>, E: Into<failure::Error>, F: FnOnce()->Result<Vec<u8>, E>>(&mut self, path: P, f: F) -> Result<Vec<u8>, failure::Error> {
    let path = path.as_ref();
    let content = match self.exists(path) {
      FileDep::Touched => {
        let content = f().map_err(|e| e.into())?;
        std::mem::replace(self, compare_and_write(path, &content)?).drop();
        content
      },
      FileDep::Unchanged => {
        load_content_raw(&path)?.ok_or_else(|| failure::err_msg("open failed"))?
      }
    };
    Ok(content)
  }
  pub fn with<E, F: FnOnce()->Result<Self, E>>(&mut self, f: F) -> Result<&mut Self, E> {
    if let FileDep::Touched = self {
      std::mem::replace(self, f()?).drop();
    }
    Ok(self)
  }
  pub fn drop(self) {}
}

pub fn load_content_raw<P: AsRef<Path>>(path: P) -> Result<Option<Vec<u8>>, failure::Error> {
  use std::io::prelude::*;
  if let Ok(mut f) = std::fs::File::open(&path) {
    let mut content = Vec::new();
    f.read_to_end(&mut content)?;
    return Ok(Some(content))
  }
  Ok(None)
}
pub fn load_content<P: AsRef<Path>>(path: P) -> Result<Option<String>, failure::Error> {
  if let Some(content) = load_content_raw(path)? {
    Ok(Some(String::from_utf8(content)?))
  } else {
    Ok(None)
  }
}
pub fn compare_and_write<P: AsRef<Path>>(path: P, content: &[u8]) -> Result<FileDep, failure::Error> {
  use std::io::prelude::*;
  use std::fs::*;
  if let Some(old_content) = load_content_raw(&path)? {
    if old_content == content {
      return Ok(FileDep::Unchanged)
    }
  }
  let lock_filename = path.as_ref().lock().ok_or_else(|| failure::err_msg("root path"))?;
  let mut f = OpenOptions::new().write(true).create_new(true).open(&lock_filename)?;
  f.write_all(content)?;
  std::fs::rename(&lock_filename, path)?;
  Ok(FileDep::Touched)
}
