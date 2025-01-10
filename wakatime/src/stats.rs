// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use crate::MARKDOWN_MARKERS;
use base::Error;
use chrono::{DateTime, Utc};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct UpdateResult {
  pub stats: String,
  pub last_update: Option<DateTime<Utc>>,
  pub current_update: DateTime<Utc>,
  pub was_updated: bool,
}

#[derive(Debug)]
pub struct StatsUpdater {
  readme_path: PathBuf,
  section_name: String,
}

impl StatsUpdater {
  pub fn new(readme_path: PathBuf, section_name: String) -> Self {
    Self {
      readme_path,
      section_name,
    }
  }

  pub async fn update(&self, new_content: &str) -> Result<UpdateResult, Error> {
    let readme = std::fs::read_to_string(&self.readme_path)?;
    let current_update = Utc::now();
    let (existing_content, last_update) = self.extract_content_and_timestamp(&readme)?;

    if self.contents_equal(existing_content.trim(), new_content.trim()) {
      return Ok(UpdateResult {
        stats: new_content.to_string(),
        last_update,
        current_update,
        was_updated: false,
      });
    }

    let updated_readme = self.replace_section(&readme, new_content, current_update)?;
    std::fs::write(&self.readme_path, updated_readme)?;

    Ok(UpdateResult {
      stats: new_content.to_string(),
      last_update,
      current_update,
      was_updated: true,
    })
  }

  fn extract_content_and_timestamp(
    &self,
    readme: &str,
  ) -> Result<(String, Option<DateTime<Utc>>), Error> {
    let start_marker = format!("<!--START_SECTION:{}-->", self.section_name);
    let end_marker = format!("<!--END_SECTION:{}-->", self.section_name);

    let section = readme
      .split(&start_marker)
      .nth(1)
      .and_then(|s| s.split(&end_marker).next())
      .ok_or_else(|| Error::ParseError("Failed to extract content section".into()))?;

    let last_update = self.parse_last_update(section);

    let content = section
      .lines()
      .filter(|line| !line.trim().starts_with("<!--") && !line.trim().ends_with("-->"))
      .collect::<Vec<_>>()
      .join("\n")
      .trim()
      .to_string();

    Ok((content, last_update))
  }

  fn contents_equal(&self, content1: &str, content2: &str) -> bool {
    let normalize = |s: &str| {
      s.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("```") && !line.ends_with("```"))
        .collect::<Vec<_>>()
        .join("\n")
    };

    normalize(content1) == normalize(content2)
  }

  fn replace_section(
    &self,
    readme: &str,
    content: &str,
    timestamp: DateTime<Utc>,
  ) -> Result<String, Error> {
    let start_marker = format!("<!--START_SECTION:{}-->", self.section_name);
    let end_marker = format!("<!--END_SECTION:{}-->", self.section_name);

    let new_section = format!(
      "{}\n{}{}-->\n{}\n{}",
      start_marker,
      MARKDOWN_MARKERS.last_update_prefix,
      timestamp.format(MARKDOWN_MARKERS.datetime_format),
      content,
      end_marker
    );

    let pattern = format!(
      "{}[\\s\\S]+?{}",
      regex::escape(&start_marker),
      regex::escape(&end_marker)
    );

    let re = regex::Regex::new(&pattern)?;
    Ok(re.replace(readme, &new_section).into_owned())
  }

  fn parse_last_update(&self, content: &str) -> Option<DateTime<Utc>> {
    content
      .lines()
      .find(|line| line.starts_with(MARKDOWN_MARKERS.last_update_prefix))
      .and_then(|line| {
        let timestamp = line
          .trim_start_matches(MARKDOWN_MARKERS.last_update_prefix)
          .trim_end_matches(MARKDOWN_MARKERS.html_comment_end)
          .trim();

        DateTime::parse_from_str(
          &format!("{} +0000", timestamp),
          &format!("{} %z", MARKDOWN_MARKERS.datetime_format),
        )
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
      })
  }
}
