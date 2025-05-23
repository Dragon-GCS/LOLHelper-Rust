use chrono::{DateTime, Utc};
use log4rs::append::Append;
use log4rs::config::{Deserialize, Deserializers};
use std::{collections::VecDeque, sync::RwLock};

#[derive(Debug)]
struct Record {
    level: String,
    time: DateTime<Utc>,
    message: String,
}

static LOGS: RwLock<VecDeque<Record>> = RwLock::new(VecDeque::new());

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
            time: Utc::now(),
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

pub fn init_logger() {
    #[cfg(debug_assertions)]
    let log_file = "src/log_config.dev.toml";
    #[cfg(not(debug_assertions))]
    let log_file = "src/log_config.toml";
    let mut deserializers = Deserializers::default();
    deserializers.insert("ui_logs", UILogsAppenderDeserializer);
    log4rs::init_file(log_file, deserializers).unwrap();
}

#[test]
fn test_init_logger() {
    init_logger();
    log::info!("This is an info message");
    log::warn!("This is a warning message");
    log::error!("This is an error message");
}
