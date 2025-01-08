// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
pub mod config;
pub mod error;
pub mod stats;
pub mod template;
pub mod wakatime;

pub use config::{Config, TimeRange, WakaTimeConfig};
pub use error::WakaError;
pub use stats::StatsGenerator;
pub use wakatime::{WakaTimeApi, WakaTimeClient};
