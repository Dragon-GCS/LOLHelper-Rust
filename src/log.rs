use chrono::{DateTime, Local};
use log::LevelFilter;
use log4rs::Config;
use log4rs::append::Append;
use log4rs::append::console::{ConsoleAppender, Target};
#[cfg(debug_assertions)]
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
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

pub fn init_logger() {
    let encoder = PatternEncoder::new(
        "{h({d(%Y-%m-%d %H:%M:%S.%f)(local):.23})} | {h({l}):>5} | {M}:{L} - {m}{n}",
    );
    let console = ConsoleAppender::builder()
        .target(Target::Stdout)
        .encoder(Box::new(encoder.clone()))
        .build();

    #[cfg(debug_assertions)]
    let (max_size, lcu_logger_level) = { (1024, LevelFilter::Trace) };
    #[cfg(not(debug_assertions))]
    let (max_size, lcu_logger_level) = { (10240, LevelFilter::Info) };

    let ui_log = UILogsAppender(max_size);
    let lcu_logger = Logger::builder()
        .appenders(vec!["console", "ui_logs"])
        .build("lcu_helper", lcu_logger_level);

    let builder = Config::builder()
        .appender(Appender::builder().build("console", Box::new(console)))
        .appender(Appender::builder().build("ui_logs", Box::new(ui_log)))
        .logger(lcu_logger);

    #[cfg(debug_assertions)]
    let builder = {
        // 添加文件记录event详情
        let log_file = FileAppender::builder()
            .encoder(Box::new(encoder))
            .build("log/debug.log")
            .unwrap();
        let log_loger = Logger::builder()
            .appender("ui_logs")
            .build("lcu_helper::log", LevelFilter::Warn);
        let client_logger = Logger::builder()
            .appender("file")
            .build("lcu_helper::lcu::client", LevelFilter::Trace);
        builder
            .appender(Appender::builder().build("file", Box::new(log_file)))
            .logger(log_loger)
            .logger(client_logger)
    };

    let config = builder
        .build(Root::builder().build(lcu_logger_level))
        .unwrap();
    log4rs::init_config(config).unwrap();
}

#[test]
fn test_init_logger() {
    init_logger();
    log::info!("This is an info message");
    log::warn!("This is a warning message");
    log::error!("This is an error message");
}
