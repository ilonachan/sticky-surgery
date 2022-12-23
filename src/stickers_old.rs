use crate::errors::Result;
use lazy_static::lazy_static;
use log::warn;
use serenity::{ http::CacheHttp, model::prelude::* };
use std::{ collections::HashMap, env, path::Path };
use url::Url;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
enum ImageSource {
  /// A file in the server's local file system (very unlikely in production)
  Local(String),
  /// An external image URL, which we just have to trust will persist
  External(String),
  /// Indicates a selfhosted guild sticker
  SelfHostedGuild(GuildId),
  /// Indicates a selfhosted personal sticker
  SelfHostedPersonal(UserId),
  /// Indicates a selfhosted sticker pack
  SelfHostedPack(String),
}

#[derive(Clone, Debug)]
pub struct Sticker {
  name: String,
  image: ImageSource,
}
impl Sticker {
  pub fn name(&self) -> String {
    self.name.clone()
  }

  pub fn image_path(&self) -> Option<String> {
    match &self.image {
      ImageSource::Local(path) => Some(path.clone()),
      ImageSource::External(_) => None,
      ImageSource::SelfHostedGuild(guild) => {
        Some(format!("stickers/guild/{}/{}.png", guild, self.name))
      }
      ImageSource::SelfHostedPersonal(user) => {
        Some(format!("stickers/user/{}/{}.png", user, self.name))
      }
      ImageSource::SelfHostedPack(pack) => {
        Some(format!("stickers/pack/{}/{}.png", pack, self.name))
      }
    }
  }

  pub fn image_url(&self) -> Option<String> {
    let hostname = env::var("HOSTNAME").ok();
    match &self.image {
      ImageSource::Local(_) => None,
      ImageSource::External(url) => Some(url.clone()),
      ImageSource::SelfHostedGuild(guild) =>
        Some(format!("http://{}/g/{}/{}.png", hostname?, guild, self.name)),
      ImageSource::SelfHostedPersonal(user) => {
        Some(format!("http://{}/u/{}/{}.png", hostname?, user, self.name))
      }
      ImageSource::SelfHostedPack(pack) => {
        Some(format!("http://{}/p/{}/{}.png", hostname?, pack, self.name))
      }
    }
  }
}

#[derive(Debug)]
pub struct StickerDbGuildData {
  pub guild_id: GuildId,
  pub stickers: Vec<Sticker>,
  enabled_packs: Vec<String>,
  /// If any roles are whitelisted, only those people can use stickers at all.
  whitelisted_roles: Vec<RoleId>,
  /// Anyone who has a blacklisted role cannot use stickers. This
  /// overrides the whitelist behavior.
  blacklisted_roles: Vec<RoleId>,
  personal_allowed: bool,
  manager_role: Option<RoleId>,
}
impl StickerDbGuildData {
  pub async fn can_use_stickers(&self, cache_http: impl CacheHttp, user: User) -> Result<bool> {
    // any blacklisted role will block the user
    for role in &self.blacklisted_roles {
      if user.has_role(&cache_http, self.guild_id, role).await? { return Ok(false); }
    }
    // if there's no whitelist, we're done.
    if self.whitelisted_roles.is_empty() { return Ok(true); }
    // any whitelisted role will allow the user
    for role in &self.whitelisted_roles {
      if user.has_role(&cache_http, self.guild_id, role).await? { return Ok(true); }
    }
    Ok(false)
  }

  pub async fn resolve_sticker(
    &self,
    cache_http: Option<impl CacheHttp>,
    sticker: String,
    user: Option<UserId>,
    packs: &HashMap<String, StickerPack>,
    user_db: Option<&StickerDbUserData>,
  ) -> Result<Option<Sticker>> {
    // first, if a user is specified, perform the necessary role lookups
    if let Some(user) = user {
      // of course, this fails if no lookup method was provided
      if let Some(ch) = &cache_http {
        if !self.can_use_stickers(ch, user.to_user(ch).await?).await? { return Ok(None); }
      } else {
        warn!(
          "no CacheHttp was provided for sticker lookup of {sticker} in guild {guild}, so no blacklist checks could be performed for user {user}",
          guild = self.guild_id
        );
      }
    }

    // if personal stickers are allowed and available, prefer them.
    if self.personal_allowed && let Some(ud) = user_db {
      return Ok(ud.resolve_sticker(sticker.clone(), packs));
    }

    // prefer guild-specific stickers over packs
    let cur = self.stickers.iter().find(|s| s.name == sticker);
    if cur.is_some() {
      return Ok(cur.cloned());
    }

    Ok(self.enabled_packs
      .iter()
      .filter_map(|pn| packs.get(pn))
      .find_map(|pack| pack.stickers.iter().find(|s| s.name == sticker))
      .cloned())
  }
}

#[derive(Debug)]
pub struct StickerDbUserData {
  pub user_id: UserId,
  pub stickers: Vec<Sticker>,
  enabled_packs: Vec<String>,
}
impl StickerDbUserData {
  pub fn resolve_sticker(
    &self,
    sticker: String,
    packs: &HashMap<String, StickerPack>
  ) -> Option<Sticker> {
    let cur = self.stickers.iter().find(|s| s.name == sticker);
    if cur.is_some() {
      return cur.cloned();
    }

    self.enabled_packs
      .iter()
      .filter_map(|pn| packs.get(pn))
      .find_map(|pack| pack.stickers.iter().find(|s| s.name == sticker))
      .cloned()
  }
}

#[derive(Debug)]
pub struct StickerPack {
  name: String,
  display_name: String,
  stickers: Vec<Sticker>,
}

#[derive(Debug)]
pub struct StickerDatabase {
  guilds: HashMap<GuildId, StickerDbGuildData>,
  private: HashMap<UserId, StickerDbUserData>,
  packs: HashMap<String, StickerPack>,
}
impl StickerDatabase {
  pub async fn resolve_sticker(
    &self,
    cache_http: Option<impl CacheHttp>,
    user: Option<UserId>,
    guild: Option<GuildId>,
    sticker: String
  ) -> Result<Option<Sticker>> {
    // if there's no guild involved, just check personal stickers.
    if guild.is_none() || self.guilds.get(&guild.unwrap()).is_none() {
      if user.is_none() {
        return Ok(None);
      }
      return Ok(match self.private.get(&user.unwrap()) {
        Some(ud) => ud.resolve_sticker(sticker.clone(), &self.packs),
        None => None,
      });
    }

    let gd = self.guilds.get(&guild.unwrap()).unwrap();
    return gd.resolve_sticker(cache_http, sticker.clone(), user, &self.packs, match user {Some(user) => self.private.get(&user) , None => None }).await;
  }
}

lazy_static! {
  pub static ref STICKER_DB: Arc<RwLock<StickerDatabase>> = Arc::new(RwLock::new({
    StickerDatabase {
      guilds: vec![StickerDbGuildData {
        guild_id: GuildId(761260439207936012),
        blacklisted_roles: vec![],
        whitelisted_roles: vec![],
        enabled_packs: vec![],
        manager_role: None,
        personal_allowed: true,
        stickers: vec![
          Sticker {
            name: "bigbrain".to_string(),
            image: ImageSource::SelfHostedGuild(GuildId(761260439207936012)),
          },
          Sticker {
            name: "headpats".to_string(),
            image: ImageSource::SelfHostedGuild(GuildId(761260439207936012)),
          }
        ],
      }]
        .drain(..)
        .map(|gd| (gd.guild_id, gd))
        .collect(),
      private: HashMap::default(),
      packs: HashMap::default(),
    }
  }));
}