use semver::range::{self, VersionReq, Op, Predicate};

#[derive(Debug, Clone)]
pub struct VersionRange(String, VersionReq);

impl serde::Serialize for VersionRange {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&self.0)
  }
}
impl<'de> serde::Deserialize<'de> for VersionRange {
  fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let s = String::deserialize(deserializer)?;
    let v = range::parse(&s).map_err(serde::de::Error::custom)?;
    Ok(Self(s, v))
  }
}
impl std::str::FromStr for VersionRange {
  type Err = String;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let v = range::parse(s).map_err(|e| e.to_string())?;
    Ok(Self(s.to_string(), v))
  }
}
impl std::fmt::Display for VersionRange {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}
impl VersionRange {
  pub fn format_version(i: &Predicate) -> String {
    match (i.minor, i.patch) {
      (Some(minor), Some(patch)) => format!("{}.{}.{}", i.major, minor, patch),
      (Some(minor), _) => format!("{}.{}", i.major, minor),
      _ => i.major.to_string(),
    }
  }
  pub fn example(&self) -> Option<String> {
    let mut result = None;
    for i in &self.1.predicates {
      match i.op {
        Op::Ex | Op::LtEq | Op::Tilde | Op::Compatible | Op::Wildcard(_) => result = Some(Self::format_version(i)),
        Op::GtEq if result.is_none() => result = Some(Self::format_version(i)),
        _ => (),
      }
    }
    result
  }
  pub fn as_coursier(&self) -> Option<String> {
    if self.0 == "*" { "latest.release".to_string().into() } else  { self.example() }
  }
}
