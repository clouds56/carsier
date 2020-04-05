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
  let p = Command::new(cmd).stdin(Stdio::null()).args(args).output()?;

  if p.status.success() {
    Ok(String::from_utf8(p.stdout)?)
  } else {
    Err(CallError::new(cmd, p))?
  }
}
