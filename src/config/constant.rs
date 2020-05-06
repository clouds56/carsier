pub const NAME: &str = "carsier";

pub const REGISTRY: &str = "crates";

pub const SCALA_VERSION: &str = "2.13";

pub fn scala_edition() -> String {
  SCALA_VERSION.to_string()
}

pub fn default_author() -> String {
  "name <email@example.com>".to_string()
}

pub fn toml_name() -> String {
  fn first_letter_to_uppper_case(s1: &str) -> String {
    let mut c = s1.chars();
    match c.next() {
      None => String::new(),
      Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
  }
  format!("{}.toml", first_letter_to_uppper_case(NAME))
}

pub fn target_dir() -> std::path::PathBuf {
  std::path::Path::new("target").to_owned()
}

pub fn default_registry() -> String {
  REGISTRY.to_string()
}

pub fn ensure_plugin() -> Result<std::path::PathBuf, anyhow::Error> {
  use std::io::Write;
  let plugin_path = std::path::Path::new("target/plugin.jar").to_owned();
  if plugin_path.exists() {
    return Ok(plugin_path)
  }
  std::fs::create_dir_all("target")?;
  let mut f = std::fs::File::create(&plugin_path)?;
  f.write(include_bytes!("../../configs/plugin.jar"))?;
  Ok(plugin_path)
}
