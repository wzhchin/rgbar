use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
    sync::Arc,
};

use arc_swap::ArcSwap;
use chin_tools::{aanyhow, AResult, EResult};
use serde::{Deserialize, Serialize};

lazy_static::lazy_static! {
    static ref CONFIG: ArcSwap<Option<ParsedConfig>> = ArcSwap::new(Arc::new(None));
}

pub fn get_config() -> Arc<Option<ParsedConfig>> {
    CONFIG.load().clone()
}

pub fn set_config() -> EResult {
    let config = Config::read_from_toml_file(None::<PathBuf>)?;
    CONFIG.store(Arc::new(Some(config)));
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub struct IconConfig {
    pub paths: Vec<String>,
    pub alias: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub struct TempConfig {
    pub always_show: Option<bool>,
    pub warn: Option<f64>,
    pub alert: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub struct Config {
    pub icon_path: String,
    pub temp: Option<TempConfig>,
}

#[derive(Debug, Clone)]
pub struct ParsedConfig {
    pub config: Config,
    pub icon: IconConfig,
}

impl Config {
    pub fn read_from_toml_file<T: AsRef<Path>>(filepath: Option<T>) -> AResult<ParsedConfig> {
        let config_path = match filepath {
            Some(fp) => fp.as_ref().to_owned(),
            None => {
                let config_dir = match env::var("XDG_CONFIG_PATH") {
                    Ok(path) => path,
                    Err(_) => format!("{}/.config", env::var("HOME").unwrap()),
                };
                let config_path = format!("{}/rgui/rgbar.toml", config_dir);
                PathBuf::from(config_path)
            }
        };

        let config_content = std::fs::read_to_string(&config_path)?;
        let config: Self = toml::from_str(config_content.as_str())?;

        let icon_path = if PathBuf::from(config.icon_path.as_str()).is_absolute() {
            config.icon_path.clone().into()
        } else {
            config_path
                .parent()
                .ok_or(aanyhow!("Parent dir is none"))?
                .join(&config.icon_path)
        };

        let config_content = std::fs::read_to_string(&icon_path)?;
        let icon_config = toml::from_str(&config_content)?;

        Ok(ParsedConfig {
            config,
            icon: icon_config,
        })
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::config::IconConfig;

    #[test]
    fn ser_test() {
        let mut map = HashMap::new();

        map.insert("key1".into(), vec!["v1".into(), "v2".into()]);
        map.insert("key2".into(), vec!["v1".into(), "v2".into()]);

        let config = IconConfig {
            paths: vec!["1".to_owned(), "2".to_owned()],
            alias: map,
        };

        println!("{:?}", toml::to_string_pretty(&config));
    }
}
