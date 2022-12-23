use sea_orm_migration::prelude::*;
use serenity::async_trait;

pub struct Migration;

impl MigrationName for Migration {
  fn name(&self) -> &str {
    "m20221222_000001_initial"
  }
}

#[async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager.create_table(
      Table::create()
        .table(Sticker::Table)
        .col(ColumnDef::new(Sticker::Id).big_unsigned().not_null().auto_increment().primary_key())
        .col(ColumnDef::new(Sticker::Name).string().not_null())
        .col(ColumnDef::new(Sticker::Creator).big_unsigned())
        .col(ColumnDef::new(Sticker::CreationDate).date_time())
        .col(ColumnDef::new(Sticker::Guild).big_unsigned())
        .col(ColumnDef::new(Sticker::User).big_unsigned())
        .col(ColumnDef::new(Sticker::Pack).big_unsigned())
        .foreign_key(
          ForeignKey::create()
            .name("fk-sticker-guild_id")
            .from(Sticker::Table, Sticker::Guild)
            .to(GuildData::Table, GuildData::Id)
            .on_update(ForeignKeyAction::Cascade)
            .on_delete(ForeignKeyAction::Cascade)
        )
        .foreign_key(
          ForeignKey::create()
            .name("fk-sticker-user_id")
            .from(Sticker::Table, Sticker::User)
            .to(UserData::Table, UserData::Id)
            .on_update(ForeignKeyAction::Cascade)
            .on_delete(ForeignKeyAction::Cascade)
        )
        .foreign_key(
          ForeignKey::create()
            .name("fk-sticker-pack_id")
            .from(Sticker::Table, Sticker::Pack)
            .to(StickerPack::Table, StickerPack::Id)
            .on_update(ForeignKeyAction::Cascade)
            .on_delete(ForeignKeyAction::Cascade)
        )
        .to_owned()
    ).await?;

    manager.create_table(
      Table::create()
        .table(StickerPack::Table)
        .col(ColumnDef::new(StickerPack::Id).big_unsigned().not_null().auto_increment().primary_key())
        .col(ColumnDef::new(StickerPack::Prefix).string().not_null())
        .col(ColumnDef::new(StickerPack::DisplayName).string())
        .col(ColumnDef::new(StickerPack::Creator).big_unsigned())
        .col(ColumnDef::new(StickerPack::CreationDate).date_time())
        .to_owned()
    ).await?;

    manager.create_table(
      Table::create()
        .table(GuildData::Table)
        .col(ColumnDef::new(GuildData::Id).big_unsigned().not_null().primary_key())
        .col(ColumnDef::new(GuildData::PersonalAllowed).boolean().not_null())
        .col(ColumnDef::new(GuildData::ManagerRole).big_unsigned())
        .to_owned()
    ).await?;

    manager.create_table(
      Table::create()
        .table(Role::Table)
        .col(ColumnDef::new(Role::Id).big_unsigned().not_null().primary_key())
        .col(ColumnDef::new(Role::Guild).big_unsigned().not_null())
        .col(ColumnDef::new(Role::Whitelisted).boolean().not_null())
        .foreign_key(
          ForeignKey::create()
            .name("fk-role-guild_id")
            .from(Role::Table, Role::Guild)
            .to(GuildData::Table, GuildData::Id)
        )
        .to_owned()
    ).await?;

    manager.create_table(
      Table::create()
        .table(GuildPackRel::Table)
        .col(ColumnDef::new(GuildPackRel::GuildId).big_unsigned().not_null())
        .col(ColumnDef::new(GuildPackRel::PackId).big_unsigned().not_null())
        .primary_key(Index::create().col(GuildPackRel::GuildId).col(GuildPackRel::PackId))
        .foreign_key(
          ForeignKey::create()
            .name("fk-guild-pack_guildid")
            .from(GuildPackRel::Table, GuildPackRel::GuildId)
            .to(GuildData::Table, GuildData::Id)
            .on_update(ForeignKeyAction::Cascade)
            .on_delete(ForeignKeyAction::Cascade)
        )
        .foreign_key(
          ForeignKey::create()
            .name("fk-guild-pack_packid")
            .from(GuildPackRel::Table, GuildPackRel::PackId)
            .to(StickerPack::Table, StickerPack::Id)
            .on_update(ForeignKeyAction::Cascade)
            .on_delete(ForeignKeyAction::Cascade)
        )
        .to_owned()
    ).await?;

    manager.create_table(
      Table::create()
        .table(UserData::Table)
        .col(ColumnDef::new(UserData::Id).big_unsigned().not_null().primary_key())
        .to_owned()
    ).await?;

    manager.create_table(
      Table::create()
        .table(UserPackRel::Table)
        .col(ColumnDef::new(UserPackRel::UserId).big_unsigned().not_null())
        .col(ColumnDef::new(UserPackRel::PackId).big_unsigned().not_null())
        .primary_key(Index::create().col(UserPackRel::UserId).col(UserPackRel::PackId))
        .foreign_key(
          ForeignKey::create()
            .name("fk-user-pack_guildid")
            .from(UserPackRel::Table, UserPackRel::UserId)
            .to(UserData::Table, UserData::Id)
            .on_update(ForeignKeyAction::Cascade)
            .on_delete(ForeignKeyAction::Cascade)
        )
        .foreign_key(
          ForeignKey::create()
            .name("fk-user-pack_packid")
            .from(UserPackRel::Table, UserPackRel::PackId)
            .to(StickerPack::Table, StickerPack::Id)
            .on_update(ForeignKeyAction::Cascade)
            .on_delete(ForeignKeyAction::Cascade)
        )
        .to_owned()
    ).await?;

    Ok(())
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager.drop_table(Table::drop().table(UserPackRel::Table).to_owned()).await?;
    manager.drop_table(Table::drop().table(GuildPackRel::Table).to_owned()).await?;

    manager.drop_table(Table::drop().table(Role::Table).to_owned()).await?;

    manager.drop_table(Table::drop().table(UserData::Table).to_owned()).await?;
    manager.drop_table(Table::drop().table(GuildData::Table).to_owned()).await?;

    manager.drop_table(Table::drop().table(StickerPack::Table).to_owned()).await?;
    manager.drop_table(Table::drop().table(Sticker::Table).to_owned()).await?;

    Ok(())
  }
}

#[derive(Iden)]
pub enum GuildData {
  Table,
  Id,
  PersonalAllowed,
  ManagerRole,
}

#[derive(Iden)]
pub enum GuildPackRel {
  Table,
  GuildId,
  PackId,
}

#[derive(Iden)]
pub enum Role {
  Table,
  Id,
  Guild,
  Whitelisted,
}

#[derive(Iden)]
pub enum UserData {
  Table,
  Id,
}
#[derive(Iden)]
pub enum UserPackRel {
  Table,
  UserId,
  PackId,
}

#[derive(Iden)]
pub enum StickerPack {
  Table,
  Id,
  Prefix,
  Creator,
  CreationDate,
  DisplayName,
}

#[derive(Iden)]
pub enum Sticker {
  Table,
  Id,
  Name,
  Creator,
  CreationDate,
  Guild,
  User,
  Pack,
}