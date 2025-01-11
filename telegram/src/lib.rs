// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
mod builders;
mod client;
mod config;
mod types;

pub use crate::{
  client::TelegramClient,
  types::{FileType, ParseMode},
};
