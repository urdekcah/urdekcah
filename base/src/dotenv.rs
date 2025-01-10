use crate::Error;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Once;

pub(in crate::dotenv) static INIT: Once = Once::new();
pub(in crate::dotenv) static DEFAULT_FILENAME: &str = ".env";

#[derive(Debug, Default)]
pub struct Dotenv {
  vars: HashMap<String, String>,
}

impl Dotenv {
  pub fn new() -> Self {
    Self {
      vars: HashMap::new(),
    }
  }

  /// Загружает переменные окружения из файла .env
  ///
  /// # Аргументы
  /// * `filename` - Необязательный путь к файлу .env. Если передано None, используется ".env" по умолчанию.
  ///
  /// # Возвращает
  /// * `Result<(), Error>` - Ok(()) в случае успеха, Error в противном случае.
  ///
  /// # Пример
  /// ```
  /// use dotenv::Dotenv;
  /// let mut config = Dotenv::new();
  /// config.load_from_file(None).expect("Не удалось загрузить файл .env");
  /// ```
  pub fn load_from_file<P: AsRef<Path>>(&mut self, filename: Option<P>) -> Result<(), Error> {
    let path = filename.map_or_else(
      || PathBuf::from(DEFAULT_FILENAME),
      |p| p.as_ref().to_path_buf(),
    );

    if !path.exists() {
      return Err(Error::PathNotFound(path));
    }

    let file = File::open(&path)?;
    let reader = BufReader::new(file);

    for (line_num, line) in reader.lines().enumerate() {
      let line = line?;
      let trimmed = line.trim();

      if trimmed.is_empty() || trimmed.starts_with('#') {
        continue;
      }

      match self.parse_line(trimmed) {
        Ok((key, value)) => {
          self.vars.insert(key, value);
        }
        Err(err) => {
          return Err(Error::Err(format!(
            "Error on line {}: {}",
            line_num + 1,
            err
          )));
        }
      }
    }

    Ok(())
  }

  fn parse_line(&self, line: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = line.splitn(2, '=').collect();

    if parts.len() != 2 {
      return Err("Invalid format: missing '='".to_string());
    }

    let key = parts[0].trim();
    let value = parts[1].trim();

    if key.is_empty() {
      return Err("Empty key".to_string());
    }

    let value = value.trim_matches('"').trim_matches('\'').to_string();

    Ok((key.to_string(), value))
  }

  pub fn set_env_vars(&self) {
    for (key, value) in &self.vars {
      env::set_var(key, value);
    }
  }

  pub fn get(&self, key: &str) -> Option<&String> {
    self.vars.get(key)
  }
}

pub fn load() -> Result<(), Error> {
  let mut result = Ok(());
  INIT.call_once(|| {
    let mut config = Dotenv::new();
    match config.load_from_file::<&str>(None) {
      Ok(()) => {
        config.set_env_vars();
      }
      Err(err) => {
        result = Err(err);
      }
    }
  });
  result
}
