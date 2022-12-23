use log::{info};

use sea_orm::DatabaseConnection;
use sea_orm_migration::prelude::*;

// Add each migration file as a module
mod m20221222_000001_initial;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            // Define the order of migrations.
            Box::new(m20221222_000001_initial::Migration),
        ]
    }
}

#[allow(dead_code)]
pub async fn recreate(db: &DatabaseConnection) -> Result<(), DbErr> {
  info!("Recreating database");
  Migrator::refresh(db).await?;
  check(db).await
}
#[allow(dead_code)]
pub async fn update(db: &DatabaseConnection) -> Result<(), DbErr> {
  let pending = Migrator::get_pending_migrations(db).await?;
  if pending.len() > 0 {
    info!("Applying {} pending migrations", pending.len());
    Migrator::up(db, None).await?;
  } else {
    info!("No migrations pending")
  }
  check(db).await
}

pub async fn check(db: &DatabaseConnection) -> Result<(), DbErr> {
  let schema_manager = SchemaManager::new(db);
  assert!(schema_manager.has_table("sticker").await?);
  assert!(schema_manager.has_table("sticker_pack").await?);
  assert!(schema_manager.has_table("user_data").await?);
  assert!(schema_manager.has_table("guild_data").await?);
  assert!(schema_manager.has_table("role").await?);
  assert!(schema_manager.has_table("user_pack_rel").await?);
  assert!(schema_manager.has_table("guild_pack_rel").await?);
  Ok(())
}