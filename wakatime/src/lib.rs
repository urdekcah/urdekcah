// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
pub mod stats;
pub mod template;
pub mod wakatime;

pub use stats::StatsGenerator;
pub use wakatime::{WakaTimeApi, WakaTimeClient};

const LAST_UPDATE_PREFIX: &str = "<!--LAST_WAKA_UPDATE:";
const HTML_COMMENT_END: &str = "-->";
const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
