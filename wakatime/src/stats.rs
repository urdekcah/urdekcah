// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use crate::{
  template::Template,
  wakatime::{WakaStats, WakaTimeApi},
};
use config::Config;
use error::Error;
use std::{fs, path::Path};
use tracing::{debug, instrument};

pub struct StatsGenerator<T: WakaTimeApi> {
  config: Config,
  client: T,
}

impl<T: WakaTimeApi> StatsGenerator<T> {
  pub fn new(config: Config, client: T) -> Self {
    Self { config, client }
  }

  #[instrument(skip(self))]
  pub async fn generate_stats(&self) -> Result<String, Error> {
    debug!("Fetching WakaTime stats");
    let stats = self
      .client
      .fetch_stats(&self.config.wakatime.time_range)
      .await?;
    debug!("Preparing content from stats");
    self.prepare_content(&stats)
  }

  #[instrument(skip(self, stats))]
  fn prepare_content(&self, stats: &WakaStats) -> Result<String, Error> {
    let template = Template::new(&self.config);
    template.render(stats)
  }

  #[instrument(skip(self, content))]
  pub fn update_readme<P: AsRef<Path> + std::fmt::Debug>(
    &self,
    path: P,
    content: &str,
  ) -> Result<bool, Error> {
    let readme = fs::read_to_string(path.as_ref())?;

    let start_comment = format!("<!--START_SECTION:{}-->", self.config.wakatime.section_name);
    let end_comment = format!("<!--END_SECTION:{}-->", self.config.wakatime.section_name);

    let replacement = format!(
      "{}\n```{}\n{}```\n{}",
      start_comment, self.config.wakatime.code_lang, content, end_comment
    );

    let pattern = format!(
      "{}[\\s\\S]+{}",
      regex::escape(&start_comment),
      regex::escape(&end_comment)
    );

    let re = regex::Regex::new(&pattern)
      .map_err(|e| Error::TemplateError(format!("Invalid regex pattern: {}", e)))?;

    let new_readme = re.replace(&readme, replacement);

    if new_readme != readme {
      fs::write(path, new_readme.as_bytes())?;
      debug!("README updated successfully");
      Ok(true)
    } else {
      debug!("No changes needed in README");
      Ok(false)
    }
  }
}
