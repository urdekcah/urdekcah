// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ParseMode {
  Markdown,
  Html,
  MarkdownV2,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum FileType {
  Document,
  Photo,
  Video,
  Audio,
}

#[derive(Debug, Serialize)]
pub(crate) struct InlineKeyboard {
  pub inline_keyboard: Vec<Vec<InlineKeyboardButton>>,
}

#[derive(Debug, Serialize)]
pub(crate) struct InlineKeyboardButton {
  pub text: String,
  pub url: String,
}

#[derive(Deserialize)]
pub(crate) struct TelegramResponse {
  pub ok: bool,
  #[serde(default)]
  pub description: String,
}

#[derive(Serialize)]
pub(crate) struct Message<'a> {
  pub chat_id: i64,
  pub text: &'a str,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub parse_mode: Option<ParseMode>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub disable_web_page_preview: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub disable_notification: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub reply_to_message_id: Option<i64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub reply_markup: Option<InlineKeyboard>,
}
