mod tts;

use anyhow::Result;
use std::io;
use std::fs::File;

use regex::Regex;

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
use songbird::tracks::PlayMode;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};

use std::env;

use crate::tts::*;

use reqwest;

#[group]
#[commands(connect, discconect)]
struct General;

#[derive(Debug, Clone)]
struct Handler {
    tx: Sender<SpeechContext>
}
impl Handler {
    fn new(tx: Sender<SpeechContext>) -> Self {
        Self {
            tx
        }
    }
}
#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let guild = msg.guild(ctx.clone()).await.unwrap();
        let current_user = ctx.cache.current_user().await;

        if guild.voice_states.get(&current_user.id).is_none() {
            return;
        };
        
        let _ = self.tx.send(SpeechContext::new(ctx, msg)).await;
    }
}

struct SpeechContext {
    msg: Message,
    ctx: Context
}
impl SpeechContext {
    fn new(ctx: Context, msg: Message) -> Self {
        Self {
            msg,
            ctx
        }
    }
    
    async fn speak(&self) {
        let guild = self.msg.guild(self.ctx.clone()).await.unwrap();
        let current_user = self.ctx.cache.current_user().await;

        if guild.voice_states.get(&current_user.id).is_none() {
            return;
        };

        let client = reqwest::Client::new();

        let member = guild.members.get(&self.msg.author.id).unwrap();

        let content = self.get_validated_msg().unwrap();
        let res = client.post("http://0.0.0.0:50021/audio_query")
            .query(&[("text", format!("{}さん: {}", member.display_name(), content)), ("speaker", "0".to_string())])
            .send().await.unwrap().text().await.unwrap();

        let res = client.post("http://0.0.0.0:50021/synthesis?speaker=0")
            .body(res)
            .send().await.unwrap();
        
        let buff = res.bytes().await.unwrap();
        
        let manager = songbird::get(&self.ctx).await
            .expect("Songbird Voice client placed in at initialisation.").clone();
        
        if let Some(handler_lock) = manager.get(guild.id) {
            let mut handler = handler_lock.lock().await;

            let mut out = File::create("speak.wav").unwrap();
            let _ = io::copy(&mut buff.as_ref(), &mut out);
            let source = songbird::ffmpeg("speak.wav").await.unwrap();
            let track_handle = handler.play_source(source);

            while let Ok(info) = track_handle.get_info().await {
                if info.playing == PlayMode::End {
                    break;
                }
            }
        }
    }

    fn get_validated_msg(&self) -> Result<String> {
        if self.msg.content.clone().len() > 140 {
            return Ok("長文略".to_string());
        }

        let mut content = self.msg.content.clone();

        // Regex for checking if the message includes URL
        let re = Regex::new(r"https?://[^\s]+").unwrap(); 
        content = re.replace_all(content.as_str(), "URL").to_string();

        Ok(content)
    }
}

struct MainThread {
    client: Client
}
impl MainThread {
    async fn new(tx: Sender<SpeechContext>, token: String) -> Self {
        let framework = StandardFramework::new()
        .configure(|c| c.prefix("cap."))
        .group(&GENERAL_GROUP);
    
        let client = Client::builder(token)
            .event_handler(Handler::new(tx))
            .framework(framework)
            .register_songbird()
            .await
            .expect("Error creating client");

        Self {
            client: client
        }
    }

    async fn start(&mut self) {
        println!("Main thread starting...");
        let _ = self.client.start().await;
    }
}

struct TtsThread {
    rx: Receiver<SpeechContext>
}
impl TtsThread {
    fn new(rx: Receiver<SpeechContext>) -> Self {
        Self {
            rx
        }
    }

    async fn start(&mut self) {
        println!("TTS thread starting...");
        loop {
            if let Some(context) = self.rx.recv().await {
                context.speak().await;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let (tx, rx) = mpsc::channel(100);
    let token = env::var("DISCORD_BOT_TOKEN").expect("token not found");

    let main_thread = async {
        let mut thread = MainThread::new(tx, token).await;
        let _ = thread.start().await;
    };

    let tts_thread = async {
        let mut thread = TtsThread::new(rx);
        let _ = thread.start().await;
    };
    
    let main_handler = tokio::spawn( main_thread);
    let tts_handler = tokio::spawn(tts_thread);

    tokio::select! {
        val = main_handler => {
            println!("MainThread downed with {:?}", val);
        }
        val = tts_handler => {
            println!("TtsThread downed with {:?}", val);
        }
    }
}