#![feature(let_chains)]

mod errors;
mod stickers_old;
mod discord;
mod db;
mod stickers;

use std::sync::Arc;

use config::Config;
use errors::Result;
use lazy_static::lazy_static;

// use log::{ debug, error, info };

lazy_static! {
  pub static ref CONFIG: Config = Config::builder()
    // Add in `./Settings.toml`
    .add_source(config::File::with_name("./config.yml"))
    // Add in settings from the environment (with a prefix of APP)
    // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
    .add_source(config::Environment::with_prefix("BOT"))
    .build()
    .unwrap();
}

#[tokio::main]
async fn main() -> Result<()> {
  log4rs::init_file("log4rs.yml", Default::default()).unwrap();

  // as far as I can tell, the DatabaseConnection is always used immutably,
  // so I don't actually need an RwLock around it (just an Arc so I can pass it around)
  let db = Arc::new(db::init("sqlite:./main.db").await?);
  discord::init(db.clone()).await?;
  Ok(())
}