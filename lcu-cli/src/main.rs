use anyhow::Ok;
use lcu_backend::CONTEXT;
use std::{
    collections::HashSet,
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tokio::{sync::RwLock, time::sleep};
use tokio_util::sync::CancellationToken;

use clap::Parser;
use log::{LevelFilter, error, info, warn};
use log4rs::{
    Config,
    append::console::ConsoleAppender,
    config::{Appender, Logger, Root},
    encode::pattern::PatternEncoder,
    init_config,
};

#[derive(clap::ValueEnum, Clone, Debug)]
enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    fn to_filter(&self) -> LevelFilter {
        match self {
            LogLevel::Off => LevelFilter::Off,
            LogLevel::Error => LevelFilter::Error,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Trace => LevelFilter::Trace,
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "lcu")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "LCU CLI Tool", long_about = None)]
struct Cli {
    #[arg(short = 'a', long, value_parser=clap::value_parser!(u8).range(0..=15), default_value_t = 3)]
    accept: u8,
    #[arg(short = 's', long)]
    send_analytics: bool,
    #[arg(short = 'p', long)]
    pick: Vec<String>,
    #[arg(short = 'l', long, value_enum, default_value_t = LogLevel::Info)]
    log_level: LogLevel,
}

async fn core(args: Cli) -> anyhow::Result<()> {
    let mut i = 0;

    while CONTEXT.auto_pick.read().unwrap().unselected.is_empty() {
        if i == 5 || !CONTEXT.listening.load(Ordering::Relaxed) {
            return Err(anyhow::anyhow!(""));
        }
        i += 1;
        info!("等待 LCU 连接...({}/5)", i);
        sleep(Duration::from_secs(1)).await;
    }

    CONTEXT
        .auto_accepted_delay
        .store(args.accept, Ordering::Relaxed);
    info!("自动接受延迟设置为 {} 秒", args.accept);
    let picks = args.pick.into_iter().collect::<HashSet<String>>();
    let champions = {
        CONTEXT
            .auto_pick
            .write()
            .unwrap()
            .unselected
            .extract_if(.., |c| picks.iter().any(|p| c.1.contains(p)))
            .collect::<Vec<_>>()
    };
    if champions.is_empty() {
        warn!("没有找到匹配的英雄，自动选择功能已禁用");
    } else {
        let auto_pick = &mut CONTEXT.auto_pick.write().unwrap();
        champions.iter().for_each(|champ| {
            info!("自动选择已启用: {}", champ.1);
        });
        auto_pick.enabled = true;
        auto_pick.selected.extend(champions);
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    let level: LevelFilter = args.log_level.to_filter();

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{h({d(%Y-%m-%dT%H:%M:%S.%f)(local):.23})} [{h({l})}] {m}{n}",
        )))
        .build();

    let config = Config::builder()
        .appender(Appender::builder().build("console", Box::new(stdout)))
        .logger(
            Logger::builder()
                .appender("console")
                .build("lcu_cli", level),
        )
        .logger(
            Logger::builder()
                .appender("console")
                .build("lcu_backend", level),
        )
        .build(Root::builder().build(level))
        .unwrap();
    init_config(config).unwrap();

    let lcu = Arc::new(RwLock::new(lcu_backend::LcuClient::default()));
    let cancel_token = Arc::new(CancellationToken::new());

    let lcu_clone = lcu.clone();
    let cancel_token_clone = cancel_token.clone();

    CONTEXT.listening.store(true, Ordering::Relaxed);
    let handle = tokio::spawn(async move {
        lcu_backend::start_event_listener(lcu_clone, cancel_token_clone)
            .await
            .unwrap_or_else(|e| {
                CONTEXT.listening.store(false, Ordering::Relaxed);
                error!("启动事件监听失败: {e}");
            });
    });
    core(args).await.ok();
    tokio::signal::ctrl_c().await.ok();
    cancel_token.cancel();
    handle.await.ok();
}
