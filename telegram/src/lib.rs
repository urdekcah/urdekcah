// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use error::Error;
use reqwest::{
  multipart::{Form, Part},
  Client,
};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};
use tokio::{fs::File, io::AsyncReadExt};
use tracing::{debug, error, instrument, warn};

const TELEGRAM_API_BASE: &str = "https://api.telegram.org/bot";
const DEFAULT_TIMEOUT_SECS: u64 = 10;
const MAX_MESSAGE_LENGTH: usize = 4096;
const DEFAULT_RETRY_ATTEMPTS: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;

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

#[derive(Debug)]
pub struct FileData<'a> {
  file_path: &'a Path,
  caption: Option<&'a str>,
  file_type: FileType,
}

#[derive(Clone, Debug)]
pub struct TelegramConfig {
  token: String,
  timeout: Duration,
  retry_attempts: u32,
  retry_delay: Duration,
}

impl Default for TelegramConfig {
  fn default() -> Self {
    Self {
      token: String::new(),
      timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
      retry_attempts: DEFAULT_RETRY_ATTEMPTS,
      retry_delay: Duration::from_millis(RETRY_DELAY_MS),
    }
  }
}

#[derive(Clone)]
pub struct TelegramClient {
  config: TelegramConfig,
  client: Client,
}

impl TelegramClient {
  pub fn builder() -> TelegramClientBuilder {
    TelegramClientBuilder::default()
  }

  pub fn message(&self) -> MessageBuilder {
    MessageBuilder::new()
  }

  pub fn file(&self) -> FileMessageBuilder {
    FileMessageBuilder::new()
  }

  #[instrument(skip(self, message), fields(chat_id = message.chat_id))]
  async fn send_message(&self, message: Message<'_>) -> Result<(), Error> {
    let url = format!("{}{}/sendMessage", TELEGRAM_API_BASE, self.config.token);

    for attempt in 0..=self.config.retry_attempts {
      match self.try_send_message(&url, &message).await {
        Ok(_) => {
          debug!("Message sent successfully");
          return Ok(());
        }
        Err(e) => {
          if attempt == self.config.retry_attempts {
            error!("All retry attempts failed");
            return Err(e);
          }
          warn!("Attempt {} failed: {}. Retrying...", attempt + 1, e);
          tokio::time::sleep(self.config.retry_delay).await;
        }
      }
    }

    Err(Error::ApiError("Max retry attempts reached".into()))
  }

  async fn try_send_message(&self, url: &str, message: &Message<'_>) -> Result<(), Error> {
    let response = self
      .client
      .post(url)
      .json(message)
      .send()
      .await
      .map_err(Error::HttpError)?;

    let status = response.status();

    if status.as_u16() == 429 {
      return Err(Error::RateLimitExceeded);
    }

    let telegram_response: TelegramResponse = response.json().await.map_err(Error::HttpError)?;

    if !telegram_response.ok {
      return Err(Error::ApiError(format!(
        "{}: {}",
        status, telegram_response.description
      )));
    }

    Ok(())
  }

  #[instrument(skip(self, file_data), fields(chat_id, file_path = %file_data.file_path.display()))]
  pub async fn send_file(&self, chat_id: i64, file_data: FileData<'_>) -> Result<(), Error> {
    let url = match file_data.file_type {
      FileType::Document => format!("{}{}/sendDocument", TELEGRAM_API_BASE, self.config.token),
      FileType::Photo => format!("{}{}/sendPhoto", TELEGRAM_API_BASE, self.config.token),
      FileType::Video => format!("{}{}/sendVideo", TELEGRAM_API_BASE, self.config.token),
      FileType::Audio => format!("{}{}/sendAudio", TELEGRAM_API_BASE, self.config.token),
    };

    for attempt in 0..=self.config.retry_attempts {
      match self.try_send_file(&url, chat_id, &file_data).await {
        Ok(_) => {
          debug!("File sent successfully");
          return Ok(());
        }
        Err(e) => {
          if attempt == self.config.retry_attempts {
            error!("All retry attempts failed");
            return Err(e);
          }
          warn!("Attempt {} failed: {}. Retrying...", attempt + 1, e);
          tokio::time::sleep(self.config.retry_delay).await;
        }
      }
    }

    Err(Error::ApiError("Max retry attempts reached".into()))
  }

  async fn try_send_file(
    &self,
    url: &str,
    chat_id: i64,
    file_data: &FileData<'_>,
  ) -> Result<(), Error> {
    let mut file = File::open(file_data.file_path)
      .await
      .map_err(Error::IoError)?;

    let file_name = file_data
      .file_path
      .file_name()
      .and_then(|n| n.to_str())
      .unwrap_or("file");

    // Read the entire file into memory
    let mut buffer = Vec::new();
    file
      .read_to_end(&mut buffer)
      .await
      .map_err(Error::IoError)?;

    let file_part = Part::bytes(buffer).file_name(file_name.to_string());

    let form = Form::new().text("chat_id", chat_id.to_string()).part(
      match file_data.file_type {
        FileType::Document => "document",
        FileType::Photo => "photo",
        FileType::Video => "video",
        FileType::Audio => "audio",
      }
      .to_string(),
      file_part,
    );

    let form = if let Some(caption) = file_data.caption {
      form.text("caption", caption.to_string())
    } else {
      form
    };

    let response = self
      .client
      .post(url)
      .multipart(form)
      .send()
      .await
      .map_err(Error::HttpError)?;

    let status = response.status();

    if status.as_u16() == 429 {
      return Err(Error::RateLimitExceeded);
    }

    let telegram_response: TelegramResponse = response.json().await.map_err(Error::HttpError)?;

    if !telegram_response.ok {
      return Err(Error::ApiError(format!(
        "{}: {}",
        status, telegram_response.description
      )));
    }

    Ok(())
  }
}

#[derive(Default)]
pub struct TelegramClientBuilder {
  config: TelegramConfig,
}

impl TelegramClientBuilder {
  pub fn token(mut self, token: impl Into<String>) -> Self {
    self.config.token = token.into();
    self
  }

  pub fn timeout(mut self, timeout: Duration) -> Self {
    self.config.timeout = timeout;
    self
  }

  pub fn retry_attempts(mut self, attempts: u32) -> Self {
    self.config.retry_attempts = attempts;
    self
  }

  pub fn retry_delay(mut self, delay: Duration) -> Self {
    self.config.retry_delay = delay;
    self
  }

  pub fn build(self) -> Result<TelegramClient, Error> {
    if self.config.token.is_empty() {
      return Err(Error::ConfigError("Bot token cannot be empty".into()));
    }

    let client = Client::builder()
      .timeout(self.config.timeout)
      .build()
      .map_err(Error::HttpError)?;

    Ok(TelegramClient {
      config: self.config,
      client,
    })
  }
}

#[derive(Serialize)]
struct Message<'a> {
  chat_id: i64,
  text: &'a str,
  #[serde(skip_serializing_if = "Option::is_none")]
  parse_mode: Option<ParseMode>,
  #[serde(skip_serializing_if = "Option::is_none")]
  disable_web_page_preview: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  disable_notification: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  reply_to_message_id: Option<i64>,
}

#[derive(Default)]
pub struct MessageBuilder<'a> {
  chat_id: Option<i64>,
  text: Option<&'a str>,
  parse_mode: Option<ParseMode>,
  disable_preview: Option<bool>,
  silent: Option<bool>,
  reply_to: Option<i64>,
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

    let message = Message {
      chat_id,
      text,
      parse_mode: self.parse_mode,
      disable_web_page_preview: self.disable_preview,
      disable_notification: self.silent,
      reply_to_message_id: self.reply_to,
    };

    client.send_message(message).await
  }
}

#[derive(Default)]
pub struct FileMessageBuilder<'a> {
  chat_id: Option<i64>,
  file_path: Option<&'a Path>,
  caption: Option<&'a str>,
  file_type: Option<FileType>,
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

  pub fn caption(mut self, text: &'a str) -> Self {
    self.caption = Some(text);
    self
  }

  pub fn file_type(mut self, file_type: FileType) -> Self {
    self.file_type = Some(file_type);
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
      caption: self.caption,
      file_type,
    };

    client.send_file(chat_id, file_data).await
  }
}

#[derive(Deserialize)]
struct TelegramResponse {
  ok: bool,
  #[serde(default)]
  description: String,
}
