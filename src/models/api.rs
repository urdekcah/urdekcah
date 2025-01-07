// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct WeatherResponse {
  pub weather: Vec<Weather>,
  pub main: MainWeather,
  pub sys: SysInfo,
  pub name: String,
  pub cod: u16,
  pub timezone: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Weather {
  pub main: String,
  pub description: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MainWeather {
  pub temp: f64,
  pub feels_like: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SysInfo {
  pub sunrise: i64,
  pub sunset: i64,
  pub country: String,
}
