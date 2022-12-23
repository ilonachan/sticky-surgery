use crate::CONFIG;
use crate::errors::{ Error, Result };
use crate::stickers_old::STICKER_DB;

use lazy_static::lazy_static;
use log::{ debug, error, info };
use regex::Regex;
use sea_orm::DatabaseConnection;
use std::{ collections::HashMap, sync::Arc };

use serenity::{
  async_trait,
  builder::CreateApplicationCommand,
  model::{
    application::interaction::{ Interaction, InteractionResponseType },
    channel::Message,
    gateway::Ready,
    prelude::{
      command::CommandOptionType,
      interaction::application_command::CommandDataOptionValue,
      *,
    },
    webhook::Webhook,
  },
  prelude::*,
};

struct WebhookCache;
impl TypeMapKey for WebhookCache {
  type Value = Arc<RwLock<HashMap<ChannelId, Webhook>>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
  async fn message(&self, ctx: Context, msg: Message) {
    if msg.author.bot {
      return;
    }
    lazy_static! {
      static ref RE: Regex = Regex::new(r"^:([a-zA-Z0-9\-_+ ]*):$").unwrap();
    }
    if let Some(m) = RE.captures(msg.content.as_str()) {
      info!("Sticker requested: :{}:", &m[1]);
      if
        let Err(why) = send_sticker(
          ctx.clone(),
          msg.channel_id,
          m[1].to_owned(),
          msg.author.id
        ).await
      {
        error!("Error sending sticker: {:?}", why)
      }
    }
    // let sticker_image = AttachmentType::from("https://url/to/image");
    // self.send_as_webhook(channel, content, user, username, avatar_url, _, Vec, _)
  }

  async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
    if let Interaction::ApplicationCommand(command) = interaction {
      debug!("Received command interaction: {:#?}", command);

      let main_response = match command.data.name.as_str() {
        "st" => {
          let option = command.data.options.get(0).expect("").resolved.as_ref().expect("");

          if let CommandDataOptionValue::String(sticker) = option {
            match
              send_sticker(
                ctx.clone(),
                command.channel_id,
                sticker.to_owned(),
                command.user.id
              ).await
            {
              Ok(_) => None,
              Err(why) => Some(format!("Error: {}", why)),
            }
          } else {
            Some("Please provide a valid sticker name".to_string())
          }
        }
        _ => Some("not implemented :(".to_string()),
      };

      if
        let Err(why) = command.create_interaction_response(&ctx.http, |response| {
          response
            .kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|message| {
              message.content(main_response.unwrap_or("\u{2764}".to_string())).ephemeral(true)
            })
        }).await
      {
        error!("Error sending response: {:?}", why)
      }
    }
  }

  async fn ready(&self, ctx: Context, ready: Ready) {
    info!("{} is connected!", ready.user.name);
    ctx.online().await;
    ctx.set_activity(Activity::watching("Sticker Surge die (#RIPBOZO)")).await;

    let guild_id = GuildId(761260439207936012);

    fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
      command
        .name("st")
        .description("Send a sticker")
        .create_option(|option| {
          option
            .name("sticker")
            .description("The sticker to send")
            .kind(CommandOptionType::String)
            .required(true)
        })
    }

    let commands = guild_id.set_application_commands(&ctx.http, |commands| {
      commands.create_application_command(register)
    }).await;

    debug!("The following slash commands are registered for the test guild: {:#?}", commands);

    // guild_id.create_application_command(&ctx.http, register);
    // Command::create_global_application_command(&ctx.http, register);
  }
}

struct WebhookIdentityDefinition {
  username: String,
  avatar_url: String,
}
impl WebhookIdentityDefinition {
  #[allow(unused)]
  fn new(username: String, avatar_url: String) -> Self {
    Self {
      username,
      avatar_url,
    }
  }
  async fn from_uid(ctx: Context, uid: UserId) -> Result<Self> {
    let user = uid.to_user(ctx).await?;
    Ok(Self {
      username: user.name.to_owned(),
      avatar_url: user.avatar_url().unwrap_or(user.default_avatar_url()),
    })
  }
}
impl Default for WebhookIdentityDefinition {
  fn default() -> Self {
    Self {
      username: "Sticky Surgery".to_string(),
      avatar_url: "Sticky Surgery avatar url".to_string(),
    }
  }
}

pub async fn init(_db: Arc<DatabaseConnection>) -> Result<()> {
  // Bot permissions: 415001537536
  let token = CONFIG.get_string("discord_token").expect("Expected a token in the environment");
  let intents =
    GatewayIntents::non_privileged() |
    GatewayIntents::GUILD_WEBHOOKS |
    GatewayIntents::GUILD_MESSAGES |
    GatewayIntents::DIRECT_MESSAGES |
    GatewayIntents::MESSAGE_CONTENT;

  let mut client = Client::builder(token, intents)
    .event_handler(Handler)
    .type_map_insert::<WebhookCache>(Arc::new(RwLock::new(HashMap::default()))).await
    .expect("Err creating client");

  {
    // Initialize the client's global data store
    // let mut data = client.data.write().await;

    // data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
  }
  client.start().await?;
  Ok(())
}

async fn send_sticker(
  ctx: Context,
  channel: ChannelId,
  sticker: String,
  user: UserId
) -> Result<Option<Message>> {
  let sticker = STICKER_DB.read().await.resolve_sticker(
    Some(&ctx),
    Some(user),
    channel
      .to_channel(&ctx).await?
      .guild()
      .map(|c| c.guild_id),
    sticker.clone()
  ).await?;
  info!("sticker resolution gave: {:?}", sticker);
  if sticker.is_none() {
    return Err(Error::Other("Sticker not available".to_string()));
  }
  let sticker = sticker.unwrap();
  let url = sticker
    .image_url()
    .unwrap_or(sticker.image_path().ok_or(Error::Other("".to_string()))?);
  let attachment: AttachmentType = url.as_str().into();

  send_as_webhook(
    ctx.clone(),
    channel,
    None,
    WebhookIdentityDefinition::from_uid(ctx.clone(), user).await.unwrap(),
    vec![attachment]
  ).await
}
async fn send_as_webhook(
  ctx: Context,
  channel: ChannelId,
  content: Option<String>,
  user: WebhookIdentityDefinition,
  attachments: Vec<AttachmentType<'_>>
) -> Result<Option<Message>> {
  let webhook = ensure_webhook_by_channel_id(ctx.clone(), channel).await?;

  match
    webhook.execute(&ctx.http, false, |w| {
      let mut w = w.username(user.username).avatar_url(user.avatar_url);
      if let Some(c) = content {
        w = w.content(c);
      }
      w.files(attachments)
    }).await
  {
    Ok(ok) => Ok(ok),
    Err(err) => Err(err.into()),
  }
}

async fn ensure_webhook_by_channel_id(ctx: Context, chid: ChannelId) -> Result<Webhook> {
  let whmap_lock = {
    let data_read = ctx.data.read().await;
    data_read.get::<WebhookCache>().expect("Expected to find the Webhook Cache").clone()
  };

  {
    let mut whmap = whmap_lock.write().await;
    if !whmap.contains_key(&chid) {
      let all_webhooks = chid.webhooks(&ctx.http).await?;
      let hook_name = format!("stickysurgery-{chid}");
      for wh in all_webhooks {
        if wh.name.as_ref() == Some(&hook_name) {
          info!("Webhook for channel {chid} found, reusing");
          whmap.insert(chid, wh);
          break;
        }
      }
      if !whmap.contains_key(&chid) {
        info!("No existing webhook was found for channel {chid}, creating one");
        whmap.insert(chid, chid.create_webhook(&ctx.http, hook_name).await?);
      }
    }
  }
  let whmap = whmap_lock.read().await;
  if !whmap.contains_key(&chid) {
    return Err(
      Error::Other(format!("After multiple attempts, no webhook was created for channel {chid}"))
    );
  }
  Ok(whmap.get(&chid).unwrap().clone())
}