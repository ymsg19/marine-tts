mod commands;

use std::io;
use std::fs::File;

use serenity::async_trait;
use serenity::prelude::*;
use serenity::client::{Client, EventHandler};
use serenity::framework::standard::{
    StandardFramework,
    macros::{
        group
    }
};

use serenity::model::channel::Message;

use dotenv::dotenv;

use songbird::SerenityInit;

use std::env;

use crate::commands::tts::*;

use reqwest;

#[group]
#[commands(connect, discconect)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let guild = msg.guild(ctx.clone()).await.unwrap();
        let current_user = ctx.cache.current_user().await;
        let voice_state = guild.voice_states.get(&current_user.id).unwrap();
        if voice_state.channel_id.is_some() {
            let client = reqwest::Client::new();

            let member = guild.members.get(&msg.author.id).unwrap();

            let res = client.post("http://0.0.0.0:50021/audio_query")
                .query(&[("text", format!("{}さん: {}", member.display_name(), msg.content)), ("speaker", "0".to_string())])
                .send().await.unwrap().text().await.unwrap();

            let res = client.post("http://0.0.0.0:50021/synthesis?speaker=0")
                .body(res)
                .send().await.unwrap();
            
            let buff = res.bytes().await.unwrap();
            
            let manager = songbird::get(&ctx).await
                .expect("Songbird Voice client placed in at initialisation.").clone();
            
            if let Some(handler_lock) = manager.get(guild.id) {
                let mut handler = handler_lock.lock().await;

                let mut out = File::create("speak.wav").unwrap();
                let _ = io::copy(&mut buff.as_ref(), &mut out);
                let source = songbird::ffmpeg("speak.wav").await.unwrap();
                handler.play_source(source);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("cap."))
        .group(&GENERAL_GROUP);

    let token = env::var("DISCORD_BOT_TOKEN").expect("token");
    let mut client = Client::builder(token)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}