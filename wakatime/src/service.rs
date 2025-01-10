// Авторские права (c) 2025 urdekcah. Все права защищены.
//
// Этот исходный код распространяется под лицензией AGPL-3.0,
// текст которой находится в файле LICENSE в корневом каталоге данного проекта.
use crate::{
  stats::{StatsUpdater, UpdateResult},
  template::Template,
  wakatime::WakaTimeApi,
};
use base::{Config, Error};
use tracing::{debug, info, instrument};

pub struct WakaTimeService {
  config: Config,
  client: Box<dyn WakaTimeApi>,
  stats_updater: StatsUpdater,
}

impl WakaTimeService {
  pub fn new(config: Config, api_key: String) -> Self {
    let client = Box::new(crate::wakatime::WakaTimeClient::new(&api_key));
    let stats_updater = StatsUpdater::new("README.md".into(), config.wakatime.section_name.clone());

    Self {
      config,
      client,
      stats_updater,
    }
  }

  #[cfg(test)]
  pub fn with_client(config: Config, client: Box<dyn WakaTimeApi>) -> Self {
    let stats_updater = StatsUpdater::new("README.md".into(), config.wakatime.section_name.clone());

    Self {
      config,
      client,
      stats_updater,
    }
  }

  #[instrument(skip(self))]
  pub async fn run(&self) -> Result<UpdateResult, Error> {
    info!("Starting WakaTime stats update");

    let stats = self
      .client
      .fetch_stats(&self.config.wakatime.time_range)
      .await?;

    let template = Template::new(self.config.clone());
    let content = template.render(&stats)?;

    debug!("Updating README with new stats");
    let result = self.stats_updater.update(&content).await?;

    Ok(result)
  }
}
