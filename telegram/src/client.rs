// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use crate::{
  builders::{FileData, FileMessageBuilder, MessageBuilder, TelegramClientBuilder},
  config::{TelegramConfig, TELEGRAM_API_BASE},
  types::{FileType, InlineKeyboard, InlineKeyboardButton, Message, TelegramResponse},
};
use error::Error;
use reqwest::{
  multipart::{Form, Part},
  Client,
};
use tokio::{fs::File, io::AsyncReadExt};
use tracing::{debug, error, instrument, warn};

#[derive(Clone)]
pub struct TelegramClient {
  pub(crate) config: TelegramConfig,
  pub(crate) client: Client,
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
  pub(crate) async fn send_message(&self, message: Message<'_>) -> Result<(), Error> {
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
  pub(crate) async fn send_file(&self, chat_id: i64, file_data: FileData<'_>) -> Result<(), Error> {
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
      .file_name
      .as_ref()
      .map(|s| s.as_str())
      .unwrap_or_else(|| {
        file_data
          .file_path
          .file_name()
          .and_then(|n| n.to_str())
          .unwrap_or("file")
      });

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

    let mut form = if let Some(caption) = file_data.caption {
      form.text("caption", caption.to_string())
    } else {
      form
    };

    if !file_data.buttons.is_empty() {
      let reply_markup = InlineKeyboard {
        inline_keyboard: file_data
          .buttons
          .iter()
          .map(|row| {
            row
              .iter()
              .map(|(text, url)| InlineKeyboardButton {
                text: text.clone(),
                url: url.clone(),
              })
              .collect()
          })
          .collect(),
      };

      form = form.text(
        "reply_markup",
        serde_json::to_string(&reply_markup).expect("Failed to serialize reply markup"),
      );
    }

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
