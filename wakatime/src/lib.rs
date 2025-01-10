// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use std::time::Duration;

mod service;
pub mod stats;
pub mod template;
pub mod wakatime;

pub use service::*;

const API_TIMEOUT: Duration = Duration::from_secs(30);
const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
const MARKDOWN_MARKERS: MarkdownMarkers = MarkdownMarkers {
  last_update_prefix: "<!--LAST_WAKA_UPDATE:",
  html_comment_end: "-->",
  datetime_format: "%Y-%m-%d %H:%M:%S",
};

#[derive(Debug, Clone, Copy)]
pub struct MarkdownMarkers {
  pub last_update_prefix: &'static str,
  pub html_comment_end: &'static str,
  pub datetime_format: &'static str,
}
