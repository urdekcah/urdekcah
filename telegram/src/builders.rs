// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use crate::{
  client::TelegramClient,
  config::{TelegramConfig, MAX_MESSAGE_LENGTH},
  types::{FileType, InlineKeyboard, InlineKeyboardButton, Message, ParseMode},
};
use error::Error;
use std::path::Path;

#[derive(Default)]
pub struct MessageBuilder<'a> {
  pub(crate) chat_id: Option<i64>,
  pub(crate) text: Option<&'a str>,
  pub(crate) parse_mode: Option<ParseMode>,
  pub(crate) disable_preview: Option<bool>,
  pub(crate) silent: Option<bool>,
  pub(crate) reply_to: Option<i64>,
  pub(crate) buttons: Vec<Vec<(String, String)>>,
}

impl<'a> MessageBuilder<'a> {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn chat_id(mut self, id: i64) -> Self {
    self.chat_id = Some(id);
    self
  }

  pub fn text(mut self, text: &'a str) -> Self {
    self.text = Some(text);
    self
  }

  pub fn parse_mode(mut self, mode: ParseMode) -> Self {
    self.parse_mode = Some(mode);
    self
  }

  pub fn disable_preview(mut self) -> Self {
    self.disable_preview = Some(true);
    self
  }

  pub fn silent(mut self) -> Self {
    self.silent = Some(true);
    self
  }

  pub fn reply_to(mut self, message_id: i64) -> Self {
    self.reply_to = Some(message_id);
    self
  }

  pub fn button(mut self, buttons: Vec<(impl Into<String>, impl Into<String>)>) -> Self {
    let row = buttons
      .into_iter()
      .map(|(text, url)| (text.into(), url.into()))
      .collect();
    self.buttons.push(row);
    self
  }

  pub async fn send(self, client: &TelegramClient) -> Result<(), Error> {
    let chat_id = self
      .chat_id
      .ok_or_else(|| Error::ApiError("Chat ID is required".into()))?;

    let text = self
      .text
      .ok_or_else(|| Error::ApiError("Message text is required".into()))?;

    if text.len() > MAX_MESSAGE_LENGTH {
      return Err(Error::ApiError(format!(
        "Message too long: {} characters (max {})",
        text.len(),
        MAX_MESSAGE_LENGTH
      )));
    }

    let reply_markup = if !self.buttons.is_empty() {
      Some(InlineKeyboard {
        inline_keyboard: self
          .buttons
          .into_iter()
          .map(|row| {
            row
              .into_iter()
              .map(|(text, url)| InlineKeyboardButton { text, url })
              .collect()
          })
          .collect(),
      })
    } else {
      None
    };

    let message = Message {
      chat_id,
      text,
      parse_mode: self.parse_mode,
      disable_web_page_preview: self.disable_preview,
      disable_notification: self.silent,
      reply_to_message_id: self.reply_to,
      reply_markup,
    };

    client.send_message(message).await
  }
}

#[derive(Default)]
pub struct FileMessageBuilder<'a> {
  pub(crate) chat_id: Option<i64>,
  pub(crate) file_path: Option<&'a Path>,
  pub(crate) file_name: Option<String>,
  pub(crate) caption: Option<&'a str>,
  pub(crate) file_type: Option<FileType>,
  pub(crate) buttons: Vec<Vec<(String, String)>>,
}

impl<'a> FileMessageBuilder<'a> {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn chat_id(mut self, id: i64) -> Self {
    self.chat_id = Some(id);
    self
  }

  pub fn file(mut self, path: &'a Path) -> Self {
    self.file_path = Some(path);
    self
  }

  pub fn file_name(mut self, name: impl Into<String>) -> Self {
    self.file_name = Some(name.into());
    self
  }

  pub fn caption(mut self, text: &'a str) -> Self {
    self.caption = Some(text);
    self
  }

  pub fn file_type(mut self, file_type: FileType) -> Self {
    self.file_type = Some(file_type);
    self
  }

  pub fn buttons(mut self, buttons: Vec<(impl Into<String>, impl Into<String>)>) -> Self {
    let row = buttons
      .into_iter()
      .map(|(text, url)| (text.into(), url.into()))
      .collect();
    self.buttons.push(row);
    self
  }

  pub async fn send(self, client: &TelegramClient) -> Result<(), Error> {
    let chat_id = self
      .chat_id
      .ok_or_else(|| Error::ApiError("Chat ID is required".into()))?;

    let file_path = self
      .file_path
      .ok_or_else(|| Error::ApiError("File path is required".into()))?;

    let file_type = self
      .file_type
      .ok_or_else(|| Error::ApiError("File type is required".into()))?;

    let file_data = FileData {
      file_path,
      file_name: self.file_name,
      caption: self.caption,
      file_type,
      buttons: self.buttons,
    };

    client.send_file(chat_id, file_data).await
  }
}

#[derive(Debug)]
pub(crate) struct FileData<'a> {
  pub file_path: &'a Path,
  pub file_name: Option<String>,
  pub caption: Option<&'a str>,
  pub file_type: FileType,
  pub buttons: Vec<Vec<(String, String)>>,
}

#[derive(Default)]
pub struct TelegramClientBuilder {
  pub(crate) config: TelegramConfig,
}

impl TelegramClientBuilder {
  pub fn token(mut self, token: impl Into<String>) -> Self {
    self.config.token = token.into();
    self
  }

  pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
    self.config.timeout = timeout;
    self
  }

  pub fn retry_attempts(mut self, attempts: u32) -> Self {
    self.config.retry_attempts = attempts;
    self
  }

  pub fn retry_delay(mut self, delay: std::time::Duration) -> Self {
    self.config.retry_delay = delay;
    self
  }

  pub fn build(self) -> Result<TelegramClient, Error> {
    if self.config.token.is_empty() {
      return Err(Error::ConfigError("Bot token cannot be empty".into()));
    }

    let client = reqwest::Client::builder()
      .timeout(self.config.timeout)
      .build()
      .map_err(Error::HttpError)?;

    Ok(TelegramClient {
      config: self.config,
      client,
    })
  }
}
