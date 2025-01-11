// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use crate::Error;
use async_trait::async_trait;
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
struct TranslationResponse {
  translations: Vec<Translation>,
}

#[derive(Deserialize, Debug)]
struct Translation {
  text: String,
}

#[derive(Serialize, Debug)]
struct TranslationRequest<'a> {
  text: Vec<&'a str>,
  target_lang: String,
  source_lang: Option<String>,
}

#[async_trait]
pub trait TranslationService {
  async fn translate_batch(
    &self,
    texts: Vec<String>,
    target_lang: &str,
    source_lang: Option<&str>,
  ) -> Result<Vec<Option<String>>, Error>;
}

#[derive(Clone, Debug)]
pub struct DeepLClient {
  api_key: String,
  client: Client,
  base_url: String,
}

impl DeepLClient {
  pub fn new(api_key: impl Into<String>, is_pro: bool) -> Self {
    let api_key = api_key.into();
    let base_url = if is_pro {
      "https://api.deepl.com/v2"
    } else {
      "https://api-free.deepl.com/v2"
    };

    let mut headers = header::HeaderMap::new();
    headers.insert(
      "Authorization",
      header::HeaderValue::from_str(&format!("DeepL-Auth-Key {}", api_key))
        .expect("Invalid API key format"),
    );

    let client = Client::builder()
      .default_headers(headers)
      .build()
      .expect("Failed to create HTTP client");

    Self {
      api_key,
      client,
      base_url: base_url.to_string(),
    }
  }

  fn validate_config(&self) -> Result<(), Error> {
    if self.api_key.is_empty() {
      return Err(Error::InvalidApiKey);
    }
    Ok(())
  }
}

#[async_trait]
impl TranslationService for DeepLClient {
  async fn translate_batch(
    &self,
    texts: Vec<String>,
    target_lang: &str,
    source_lang: Option<&str>,
  ) -> Result<Vec<Option<String>>, Error> {
    self.validate_config()?;

    if texts.is_empty() {
      return Ok(Vec::new());
    }

    let request_body = TranslationRequest {
      text: texts.iter().map(|s| s.as_str()).collect(),
      target_lang: target_lang.to_uppercase(),
      source_lang: source_lang.map(|s| s.to_uppercase()),
    };

    let response = self
      .client
      .post(format!("{}/translate", self.base_url))
      .json(&request_body)
      .send()
      .await?;

    match response.status() {
      reqwest::StatusCode::OK => {
        let response_data: TranslationResponse = response
          .json()
          .await
          .map_err(|e| Error::ApiError(e.to_string()))?;

        Ok(
          response_data
            .translations
            .into_iter()
            .map(|t| Some(t.text))
            .collect(),
        )
      }
      reqwest::StatusCode::TOO_MANY_REQUESTS => Err(Error::RateLimitExceeded),
      _ => {
        let error_text = response
          .text()
          .await
          .unwrap_or_else(|_| "Unknown error".to_string());
        Err(Error::ApiError(error_text))
      }
    }
  }
}
