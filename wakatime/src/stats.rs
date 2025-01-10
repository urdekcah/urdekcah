// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use crate::{template::Template, wakatime::WakaTimeApi, MARKDOWN_MARKERS};
use base::{Config, Error};
use chrono::{DateTime, Utc};
use regex::Regex;
use std::{fs, path::Path};
use tracing::{debug, instrument};

#[derive(Debug, Clone)]
pub struct UpdateResult {
  pub stats: String,
  pub last_update: Option<DateTime<Utc>>,
  pub current_update: DateTime<Utc>,
  pub was_updated: bool,
}

pub struct StatsGenerator<T: WakaTimeApi> {
  config: Config,
  client: T,
  template: Template,
}

impl<T: WakaTimeApi> StatsGenerator<T> {
  pub fn new(config: Config, client: T) -> Self {
    let template = Template::new(config.clone());
    Self {
      config,
      client,
      template,
    }
  }

  #[instrument(skip(self))]
  pub async fn generate(&self) -> Result<UpdateResult, Error> {
    debug!("Fetching WakaTime stats");
    let stats = self
      .client
      .fetch_stats(&self.config.wakatime.time_range)
      .await?;

    debug!("Preparing content from stats");
    let content = self.template.render(&stats)?;

    self.update_readme(Path::new("README.md"), &content)
  }

  fn update_readme<P: AsRef<Path> + std::fmt::Debug>(
    &self,
    path: P,
    content: &str,
  ) -> Result<UpdateResult, Error> {
    let readme = fs::read_to_string(path.as_ref())?;
    let last_update = Self::parse_last_update(&readme);
    let current_update = Utc::now();

    let start_comment = format!("<!--START_SECTION:{}-->", self.config.wakatime.section_name);
    let end_comment = format!("<!--END_SECTION:{}-->", self.config.wakatime.section_name);

    let replacement = format!(
      "{}\n{}{}-->\n```{}\n{}```\n{}",
      start_comment,
      MARKDOWN_MARKERS.last_update_prefix,
      current_update.format(MARKDOWN_MARKERS.datetime_format),
      self.config.wakatime.code_lang,
      content,
      end_comment
    );

    let pattern = format!(
      "{}[\\s\\S]+{}",
      regex::escape(&start_comment),
      regex::escape(&end_comment)
    );

    let re = Regex::new(&pattern)
      .map_err(|e| Error::TemplateError(format!("Invalid regex pattern: {}", e)))?;

    let new_readme = re.replace(&readme, replacement);
    let was_updated = new_readme != readme;

    if was_updated {
      fs::write(path, new_readme.as_bytes())?;
      debug!("README updated successfully");
    } else {
      debug!("No changes needed in README");
    }

    Ok(UpdateResult {
      stats: content.to_string(),
      last_update,
      current_update,
      was_updated,
    })
  }

  fn parse_last_update(content: &str) -> Option<DateTime<Utc>> {
    content
      .find(MARKDOWN_MARKERS.last_update_prefix)
      .and_then(|pos| {
        let timestamp_start = pos + MARKDOWN_MARKERS.last_update_prefix.len();
        content[timestamp_start..]
          .find(MARKDOWN_MARKERS.html_comment_end)
          .map(|end| content[timestamp_start..timestamp_start + end].trim())
      })
      .and_then(|timestamp| {
        DateTime::parse_from_str(
          &format!("{} +0000", timestamp),
          &format!("{} %z", MARKDOWN_MARKERS.datetime_format),
        )
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
      })
  }
}
