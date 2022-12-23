mod migrator;
pub mod entities;

use log::trace;

use sea_orm::*;
use crate::errors::Result;

pub async fn init(db_url: &str) -> Result<DatabaseConnection> {
  let db = Database::connect(db_url).await?;
  migrator::update(&db).await?;
  trace!("database obtained");

  Ok(db)
}