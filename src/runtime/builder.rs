//! Application builder helpers.

use crate::config::Config;

use super::{Application, Lifecycle};

/// Small builder façade over the runtime application container.
#[derive(Default)]
pub struct ApplicationBuilder {
    config: Option<Config>,
    application: Option<Application>,
}

impl ApplicationBuilder {
    /// Create a new builder instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach config used to initialize the application.
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    /// Override the application instance before building.
    pub fn with_application(mut self, application: Application) -> Self {
        self.application = Some(application);
        self
    }

    /// Build the runtime application.
    pub async fn build(self) -> anyhow::Result<Application> {
        let mut application = self.application.unwrap_or_default();
        if let Some(config) = self.config {
            application.config = Some(config);
        }
        application.initialize().await?;
        Ok(application)
    }
}
