mod trade;

use binance::*;
use clap::Parser;
use log::{error, info};
use logger::*;
use serde::Deserialize;
use std::sync::Arc;
use trade::SpotTrade;
use xcrypto::rest::Rest;

#[derive(Debug, Deserialize)]
struct Config {
    margin: bool,
    apikey: String,
    pem: String,
    local: String,
    test: Option<bool>, // Add test field with default None
}

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    #[arg(short, long, help = "Config path")]
    config: String,
    #[arg(short, long, default_value_t = Level::Info)]
    level: Level,
}

impl Args {
    pub fn load(&self) -> anyhow::Result<Config> {
        info!("Load config from {}", self.config);
        let buf = std::fs::read_to_string(self.config.clone())?;
        let config: Config = native_json::parse(&buf)?;
        Ok(config)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = args.load()?;

    let path = std::env::current_exe()?;
    let filename = match path.file_name() {
        Some(name) => name.to_string_lossy(),
        None => "unknown".into(),
    };

    let _logger = init(Some(format!("log/{}", filename)), args.level);

    let app = Application::new(&config.local).await?;
    
    // Select URLs based on test configuration
    let (ws_url, api_url): (String, String) = if config.test.unwrap_or(true) {
        ("wss://stream.testnet.binance.vision/ws".into(), "https://testnet.binance.vision".into())
    } else {
        ("wss://stream.binance.com:9443/ws".into(), "https://api.binance.com".into())
    };
    
    let market = Market::new(ws_url.clone()).await?;

    let rest = Arc::new(Rest::new(
        &api_url,
        &config.apikey,
        &config.pem,
        3000,
    )?);

    let account = if config.margin {
        Account::<SpotListenKey>::new(
            &ws_url,
            "/sapi/v1/userDataStream",
            rest.clone(),
        )
        .await?
    } else {
        Account::<SpotListenKey>::new(
            &ws_url,
            "/api/v3/userDataStream",
            rest.clone(),
        )
        .await?
    };
    let trade = SpotTrade::new(rest.clone(), account, config.margin).await?;

    if let Err(e) = app.keep_running(market, trade).await {
        error!("{}", e);
    }
    Ok(())
}
