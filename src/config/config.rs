use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: Server,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Server {
    pub buffer_size: usize,
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = env::var("CONFIG_PATH")
            .with_context(|| "Переменная окружения CONFIG_PATH не задана")?;

        let data = fs::read_to_string(&config_path)
            .with_context(|| format!("Не удалось прочитать конфиг {:?}", config_path))?;

        let cfg: Config = serde_yaml::from_str(&data)
            .with_context(|| format!("Ошибка парсинга конфига {:?}", config_path))?;

        Ok(cfg)
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    pub fn buf(&self) -> Vec<u8> {
        vec![0u8; self.server.buffer_size]
    }
}
