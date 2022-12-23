use std::sync::Arc;

use sea_orm::{ DatabaseConnection, ColumnTrait, EntityTrait, QueryFilter, * };
use serenity::http::CacheHttp;
use serenity::model::prelude::{ GuildId, UserId };
use serenity::prelude::RwLock;

use crate::db::entities::{ prelude::*, * };
use crate::errors::Result;

pub enum StickerSource {
  Guild(GuildId),
  User(UserId),
  Pack(u64),
}
pub struct LSticker {
  pub id: u64,
  pub name: String,
  pub source: StickerSource,
}
impl From<&sticker::Model> for LSticker {
  fn from(value: &sticker::Model) -> Self {
    LSticker {
      id: value.id,
      name: value.name.clone(),
      source: {
        if let Some(gid) = value.guild {
          StickerSource::Guild(GuildId(gid))
        } else if let Some(uid) = value.user {
          StickerSource::User(UserId(uid))
        } else if let Some(pid) = value.pack {
          StickerSource::Pack(pid)
        } else {
          panic!()
        }
      },
    }
  }
}

struct StickerDatabase<CH: CacheHttp> {
  db: Arc<DatabaseConnection>,
  cache_http: Arc<CH>,
}
impl <CH: CacheHttp> StickerDatabase<CH> {
  pub async fn get_stickers_for_guild(&self, guild: GuildId) -> Result<Vec<LSticker>> {
    Ok(
      Sticker::find()
        .filter(sticker::Column::Guild.eq(guild.0))
        .all(self.db.as_ref()).await?
        .iter()
        .map(|st| st.into())
        .collect()
    )
  }
  pub async fn get_stickers_for_user(&self, user: UserId) -> Result<Vec<LSticker>> {
    Ok(
      Sticker::find()
        .filter(sticker::Column::User.eq(user.0))
        .all(self.db.as_ref()).await?
        .iter()
        .map(|st| st.into())
        .collect()
    )
  }
  pub async fn get_stickers_for_pack(&self, pack: String) -> Result<Vec<LSticker>> {
    Ok(
      Sticker::find()
        .inner_join(sticker_pack::Entity)
        .filter(sticker_pack::Column::Prefix.eq(pack))
        .all(self.db.as_ref()).await?
        .iter()
        .map(|st| st.into())
        .collect()
    )
  }

  pub async fn get_packs_for_guild(&self, guild: GuildId) -> Result<Vec<sticker_pack::Model>> {
    Ok(
      StickerPack::find()
        .inner_join(guild_data::Entity)
        .filter(guild_data::Column::Id.eq(guild.0))
        .all(self.db.as_ref()).await?
      // .iter()
      // .map(|st| st.into())
      // .collect()
    )
  }
  pub async fn get_packs_for_user(&self, user: UserId) -> Result<Vec<sticker_pack::Model>> {
    Ok(
      StickerPack::find()
        .inner_join(user_data::Entity)
        .filter(user_data::Column::Id.eq(user.0))
        .all(self.db.as_ref()).await?
      // .iter()
      // .map(|st| st.into())
      // .collect()
    )
  }

  pub async fn resolve_sticker(
    &self,
    sticker: String,
    uid: UserId,
    gid: Option<GuildId>
  ) -> Result<Option<LSticker>> {
    let guild = if let Some(guild) = gid {
      GuildData::find_by_id(guild.0).one(self.db.as_ref()).await?
    } else {
      None
    };

    if let Some(guild) = guild {
      let user = uid.to_user(self.cache_http.as_ref()).await?;
      for r in 
        Role::find()
          .inner_join(guild_data::Entity)
          .filter(guild_data::Column::Id.eq(gid.unwrap().0))
          .filter(role::Column::Whitelisted.eq(false))
          .all(self.db.as_ref()).await? 
      {
        if user.has_role(self.cache_http.as_ref(), gid.unwrap(), r.id).await? { return Ok(None); }
      }
      
      let whitelist = Role::find()
          .inner_join(guild_data::Entity)
          .filter(guild_data::Column::Id.eq(gid.unwrap().0))
          .filter(role::Column::Whitelisted.eq(true))
          .all(self.db.as_ref()).await?;
      if whitelist.len() > 0 {
        let mut yes = false;
        for r in whitelist {
          if user.has_role(self.cache_http.as_ref(), gid.unwrap(), r.id).await? { yes = true; }
        }
        if yes == false {
          return Ok(None);
        }
      }

      if guild.personal_allowed {
        if let Some(st) = self.resolve_by_user(sticker.clone(), uid).await? {
          return Ok(Some(st));
        }
      }

      Ok(
        Sticker::find()
          .filter(sticker::Column::Name.eq(sticker.clone()))
          .filter(sticker::Column::Guild.eq(gid.unwrap().0))
          .one(self.db.as_ref()).await?
          .or(
            Sticker::find()
              .inner_join(sticker_pack::Entity)
              .inner_join(guild_data::Entity)
              .filter(sticker::Column::Name.eq(sticker.clone()))
              .filter(guild_data::Column::Id.eq(gid.unwrap().0))
              .one(self.db.as_ref()).await?
          )
          .map(|st| (&st).into())
      )
    } else {
      self.resolve_by_user(sticker, uid).await
    }
  }

  async fn resolve_by_user(&self, sticker: String, uid: UserId) -> Result<Option<LSticker>> {
    let user = UserData::find_by_id(uid.0).one(self.db.as_ref()).await?;
    if user.is_none() {
      return Ok(None);
    }

    Ok(
      Sticker::find()
        .filter(sticker::Column::Name.eq(sticker.clone()))
        .filter(sticker::Column::User.eq(uid.0))
        .one(self.db.as_ref()).await?
        .or(
          Sticker::find()
            .inner_join(sticker_pack::Entity)
            .inner_join(user_data::Entity)
            .filter(sticker::Column::Name.eq(sticker.clone()))
            .filter(user_data::Column::Id.eq(uid.0))
            .one(self.db.as_ref()).await?
        )
        .map(|st| (&st).into())
    )
  }
}