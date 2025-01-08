// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use anyhow::{Context, Result};
use std::{fs, path::Path};

pub async fn update_file_atomically(path: &Path, content: &str) -> Result<()> {
  let temp_path = path.with_extension("tmp");
  fs::write(&temp_path, content).context("Failed to write temporary file")?;
  fs::rename(&temp_path, path).context("Failed to update file")?;
  Ok(())
}
