// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use std::time::Duration;

pub(crate) const TELEGRAM_API_BASE: &str = "https://api.telegram.org/bot";
pub(crate) const DEFAULT_TIMEOUT_SECS: u64 = 10;
pub(crate) const MAX_MESSAGE_LENGTH: usize = 4096;
pub(crate) const DEFAULT_RETRY_ATTEMPTS: u32 = 3;
pub(crate) const RETRY_DELAY_MS: u64 = 1000;

#[derive(Clone, Debug)]
pub struct TelegramConfig {
  pub(crate) token: String,
  pub(crate) timeout: Duration,
  pub(crate) retry_attempts: u32,
  pub(crate) retry_delay: Duration,
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
