pub const NAME: &str = "carsier";

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
