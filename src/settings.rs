use std::path::PathBuf;

use anyhow::{anyhow, Result};
use config::{Config, Environment, File};
use directories::ProjectDirs;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Github {
    pub personal_access_token: String,
    pub hostname: String,
    pub username: String,
    pub proxy_url: Option<String>,
    pub queries: Vec<String>,
    pub exclude_comment_patterns: Vec<String>,
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

    fn user_config_path() -> Result<PathBuf> {
        let mut toml_path = ProjectDirs::from("", "", "prnotify")
            .ok_or_else(|| anyhow!("Could not determine default config path"))?
            .config_dir()
            .to_path_buf();
        toml_path.push("prnotify.toml");
        Ok(toml_path)
    }

    pub fn try_new() -> Result<Self> {
        let user_config_path = Self::user_config_path()?;
        let system_config_path = PathBuf::from("/etc/prnotify/prnotify.toml");

        let mut builder = Config::builder();

        if system_config_path.exists() {
            let path = system_config_path
                .to_str()
                .ok_or_else(|| anyhow!("Could not determine system config path"))?;
            builder = builder.add_source(File::with_name(path));
        }

        if user_config_path.exists() {
            let path = user_config_path
                .to_str()
                .ok_or_else(|| anyhow!("Could not determine user config path"))?;
            builder = builder.add_source(File::with_name(path));
        }

        builder = builder.add_source(
            Environment::with_prefix("PRNOTIFY")
                .separator("__")
                .list_separator(","),
        );
        builder = builder.set_default("github.hostname", "api.github.com")?;
        builder = builder.set_default("github.queries", vec!["is:open is:pr involves:@me"])?;
        builder = builder.set_default::<&str, Vec<&str>>("github.exclude_comment_patterns", vec![])?;
        let config = builder.build()?;

        let mut settings: Settings = config.try_deserialize()?;

        // normalize all the paths
        settings.cache.path = Self::normalize_path(&settings.cache.path)?;

        if let Some(firefox) = settings.firefox.as_mut() {
            firefox.cookies_file_path = Self::normalize_path(&firefox.cookies_file_path)?;
        }

        Ok(settings)
    }
}
