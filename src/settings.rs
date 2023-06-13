use anyhow::{anyhow, Result};
use config::{Config, Environment, File};
use directories::ProjectDirs;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Github {
    pub personal_access_token: String,
    pub hostname: String,
    pub username: String,
    pub queries: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Ntfy {
    pub base_url: String,
    pub topic: String,
}

#[derive(Debug, Deserialize)]
pub struct Cache {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct Firefox {
    pub cookies_file_path: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub github: Github,
    pub ntfy: Ntfy,
    pub cache: Cache,
    pub firefox: Option<Firefox>,
}

impl Settings {
    fn normalize_path(path: &str) -> Result<String> {
        Ok(shellexpand::full(path)?.into_owned())
    }

    fn default_config_path() -> Result<String> {
        let mut toml_path = ProjectDirs::from("", "", "prnotify")
            .ok_or_else(|| anyhow!("Could not determine default config path"))?
            .config_dir()
            .to_path_buf();
        toml_path.push("prnotify.toml");
        toml_path
            .to_str()
            .map(|x| x.to_owned())
            .ok_or_else(|| anyhow!("Could not determine default config path"))
    }

    pub fn try_new() -> Result<Self> {
        let config = Config::builder()
            .add_source(File::with_name(Self::default_config_path()?.as_str()))
            .add_source(Environment::with_prefix("prnotify"))
            .set_default("github.hostname", "github.com")?
            .set_default("github.queries", vec!["is:open is:pr involves:@me"])?
            .build()?;

        let mut settings: Settings = config.try_deserialize()?;

        // normalize all the paths
        settings.cache.path = Self::normalize_path(&settings.cache.path)?;

        if let Some(firefox) = settings.firefox.as_mut() {
            firefox.cookies_file_path = Self::normalize_path(&firefox.cookies_file_path)?;
        }

        Ok(settings)
    }
}
