use serenity::model::channel::Message;
use serenity::client::{Context};
use serenity::framework::standard::{
    CommandResult,
    macros::{
        command
    }
};

#[command]
#[aliases("con")]
#[only_in(guilds)]
#[description = "Connect bot."]
pub async fn connect(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = msg.channel_id.say(&ctx.http, "Connecting to tts server...");

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states.get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);
    
    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            let _ = msg.reply(ctx, "You are NOT in a voice channel").await;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();
    
    let _ = manager.join(guild_id, connect_to).await;

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            let _ = msg.reply(ctx, "Not in a voice channel").await;
            return Ok(());
        },
    };

    let mut handler = handler_lock.lock().await;

    if let Err(e) = handler.deafen(true).await {
        let _ = msg.channel_id.say(&ctx.http, format!("Failed: {:?}", e)).await;
    }

    Ok(())
}

#[command]
#[aliases("leave")]
#[only_in(guilds)]
#[description = "Disconnect bot."]
async fn discconect(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            let _ = msg.channel_id.say(&ctx.http, format!("Failed: {:?}", e)).await;
        }
    } else {
        let _ = msg.reply(ctx, "Not in a voice channel").await;
    }
    Ok(())
}