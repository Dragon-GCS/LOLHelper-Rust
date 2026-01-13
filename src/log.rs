use chrono::{DateTime, Local};
use log4rs::Config;
use log4rs::append::Append;
use log4rs::config::{Deserialize, Deserializers, RawConfig};
use std::fmt::Display;
use std::{collections::VecDeque, sync::RwLock};

#[derive(Debug)]
pub struct Record {
    level: String,
    time: DateTime<Local>,
    message: String,
}

impl Display for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} | {:<5} - {}",
            self.time.format("%Y-%m-%d %H:%M:%S.%3f"),
            self.level,
            self.message
        )
    }
}

pub static LOGS: RwLock<VecDeque<Record>> = RwLock::new(VecDeque::new());

#[derive(Debug)]
struct UILogsAppender(usize);

impl Append for UILogsAppender {
    fn append(&self, record: &log::Record) -> anyhow::Result<()> {
        let mut logs = LOGS.write().unwrap();
        if logs.len() >= self.0 {
            logs.pop_front();
        }
        logs.push_back(Record {
            level: record.level().to_string(),
            time: Local::now(),
            message: record.args().to_string(),
        });
        Ok(())
    }
    fn flush(&self) {}
}

#[derive(serde::Deserialize)]
pub struct UILogsAppenderConfig {
    pub max_size: Option<usize>,
}

#[derive(Default)]
pub struct UILogsAppenderDeserializer;

impl Deserialize for UILogsAppenderDeserializer {
    type Trait = dyn Append;

    type Config = UILogsAppenderConfig;

    fn deserialize(
        &self,
        config: UILogsAppenderConfig,
        _: &Deserializers,
    ) -> anyhow::Result<Box<Self::Trait>> {
        let max_size = config.max_size.unwrap_or(10000);
        let appender = UILogsAppender(max_size);
        Ok(Box::new(appender))
    }
}

// Copy code from log4rs/src/config/file.rs:deserialize
fn deserialize(config: &RawConfig, deserializers: &Deserializers) -> Config {
    let (appenders, mut errors) = config.appenders_lossy(deserializers);
    errors.handle();

    let (config, mut errors) = Config::builder()
        .appenders(appenders)
        .loggers(config.loggers())
        .build_lossy(config.root());

    errors.handle();

    config
}

pub fn init_logger() {
    #[cfg(debug_assertions)]
    let config_str = include_str!("./log_config.dev.toml");
    #[cfg(not(debug_assertions))]
    let config_str = include_str!("./log_config.toml");
    let mut deserializers = Deserializers::default();
    deserializers.insert("ui_logs", UILogsAppenderDeserializer);

    let raw_config: RawConfig = ::toml::from_str(config_str).unwrap();
    let config = deserialize(&raw_config, &deserializers);

    log4rs::init_config(config).unwrap();
}

#[test]
fn test_init_logger() {
    init_logger();
    log::info!("This is an info message");
    log::warn!("This is a warning message");
    log::error!("This is an error message");
}
