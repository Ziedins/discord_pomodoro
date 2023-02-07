use std::env;

use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

use crate::data::task::Task;
pub mod data;

const HELP_MESSAGE: &str = "
Hello, I'm pomodoro bot!

— PomodoroBot 🤖
";

const COMMAND_NOT_FOUND_MESSAGE: &str = "
I'm just a bot, I cannot do this
";

const HELP_COMMAND: &str = "!help";
const ADD_TASK_COMMAND: &str = "!add_task";


struct Handler;


#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == HELP_COMMAND {
            if let Err(why) = msg.channel_id.say(&ctx.http, HELP_MESSAGE).await {
                println!("Error sending message: {:?}", why);
            }
        }
        
        match_message_command(&msg.content);
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

fn match_message_command(command: &str) {
    match command {
        HELP_COMMAND => execute_help(),
        ADD_TASK_COMMAND => execute_add_task(),
        _ => (), 
    }
}

fn execute_help() {
    println!("Help test");
}

fn execute_add_task() {
    println!("Add task test");
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment");

    let mut client = Client::new(&token)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
