// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use crate::wakatime::WakaStats;
use base::{Config, Error};
use chrono::DateTime;
use std::collections::HashSet;
use tracing::{debug, instrument};

const GRAPH_WIDTH: usize = 25;
const TIME_WIDTH: usize = 16;

#[derive(Debug)]
pub struct Template {
  config: Config,
  ignored_langs: HashSet<String>,
}

impl Template {
  pub fn new(config: Config) -> Self {
    let ignored_langs = config
      .wakatime
      .ignored_languages
      .as_ref()
      .map(|s| s.split_whitespace().map(String::from).collect())
      .unwrap_or_default();

    Self {
      config,
      ignored_langs,
    }
  }

  #[instrument(skip(self, stats))]
  pub fn render(&self, stats: &WakaStats) -> Result<String, Error> {
    let mut content = String::new(); // Just use String::new() instead of with_capacity :D

    content.push_str(format!("```{}\n", self.config.wakatime.code_lang).as_str());

    if self.config.wakatime.show_title {
      content.push_str(&self.render_title(stats));
    }

    if self.config.wakatime.show_masked_time || self.config.wakatime.show_total {
      content.push_str(&self.render_total_time(stats));
    }

    content.push_str(&self.render_languages(stats));
    content.push_str("```");
    debug!("Template rendered successfully");
    Ok(content)
  }

  fn render_title(&self, stats: &WakaStats) -> String {
    let format_date = |date_str: &str| {
      DateTime::parse_from_rfc3339(date_str)
        .map(|dt| dt.format("%d %B %Y").to_string())
        .unwrap_or_else(|_| "Unknown".to_string())
    };

    format!(
      "From: {} - To: {}\n\n",
      format_date(&stats.start),
      format_date(&stats.end)
    )
  }

  fn render_total_time(&self, stats: &WakaStats) -> String {
    let total = match (
      self.config.wakatime.show_masked_time,
      self.config.wakatime.show_total,
      &stats.human_readable_total_including_other_language,
      &stats.human_readable_total,
    ) {
      (true, _, Some(total), _) => Some(total),
      (false, true, _, Some(total)) => Some(total),
      _ => None,
    };

    total
      .map(|t| format!("Total Time: {}\n\n", t))
      .unwrap_or_default()
  }

  fn render_languages(&self, stats: &WakaStats) -> String {
    let max_name_len = stats
      .languages
      .iter()
      .map(|l| l.name.len())
      .max()
      .unwrap_or(0);

    let mut content = String::with_capacity(stats.languages.len() * 64);
    let lang_count = self.config.wakatime.lang_count as usize;

    for (_idx, lang) in stats
      .languages
      .iter()
      .filter(|l| !self.ignored_langs.contains(&l.name))
      .take(if lang_count > 0 {
        lang_count
      } else {
        usize::MAX
      })
      .enumerate()
    {
      let graph = self.make_graph(lang.percent);
      let time_str = if self.config.wakatime.show_time {
        &lang.text
      } else {
        ""
      };

      content.push_str(&format!(
        "{:<name_width$}   {:<time_width$}{:<graph_width$}   {:>05.2} %\n",
        lang.name,
        time_str,
        graph,
        lang.percent,
        name_width = max_name_len,
        time_width = TIME_WIDTH,
        graph_width = GRAPH_WIDTH
      ));

      if self.config.wakatime.stop_at_other && lang.name == "Other" {
        break;
      }
    }

    content
  }

  fn make_graph(&self, percent: f64) -> String {
    let blocks: Vec<char> = self.config.wakatime.blocks.chars().collect();
    if blocks.len() != 4 {
      return "Invalid blocks configuration".to_string();
    }

    let proportion = (percent / 100.0 * GRAPH_WIDTH as f64).min(GRAPH_WIDTH as f64);
    let full_blocks = (proportion + 0.125) as usize;
    let remainder = ((proportion - full_blocks as f64) * 4.0 + 0.5) as usize;

    let mut graph = String::with_capacity(GRAPH_WIDTH);
    graph.extend(std::iter::repeat(blocks[3]).take(full_blocks));

    if remainder > 0 && remainder < blocks.len() {
      graph.push(blocks[remainder]);
    }

    graph.extend(std::iter::repeat(blocks[0]).take(GRAPH_WIDTH - graph.chars().count()));
    graph
  }
}
