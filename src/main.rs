use std::time::{Instant, Duration};
use std::{env, sync::mpsc::channel};
use std::fmt::Write as _;
use std::thread::sleep;

use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready, prelude::*},
    prelude::*,
};

use crate::data::task::Task;
use chrono;
use std::sync::mpsc::channel as SyncChannel;
use timer::Timer;
pub mod data;

const COMMAND_NOT_FOUND_MESSAGE: &str = "
I'm just a bot, I cannot do this
";

const HELP_COMMAND: &str = "!help";
const TASK_ADD_COMMAND: &str = "!task add";
const TASK_REMOVE_COMMAND: &str = "!task remove";
const TASK_LIST_COMMAND: &str = "!task list";
const POMODORO_START:&str = "!pomodoro start";
const POMODORO_PAUSE:&str = "!pomodoro pause";
const POMODORO_CHECK:&str = "!pomodoro check";

const POMODORO_TIMER_MINUTES: u64 = 25;

struct Handler {
    database: sqlx::SqlitePool
}

fn help_message() -> String {
    format!("
        Hello, I'm pomodoro bot!
        Heres a list of available commands: 
        '{}' - lists available commands,
        '{}' - adds a task to the task list,
        '{}' + 'task number' - removes and completes a specific task,
        '{}' - displays task list,
        '{}' - start the pomodoro timer,
        '{}' - pause the pomodoro timer,
        '{}' - check the pomodoro current time,

        — PomodoroBot 🤖"
    ,HELP_COMMAND, TASK_ADD_COMMAND, TASK_REMOVE_COMMAND, TASK_LIST_COMMAND, POMODORO_START, POMODORO_CHECK, POMODORO_PAUSE)
}

pub struct PomodoroSession {
    pomodoro_tracker: StateTracker,
    clock: Clock,
}

impl PomodoroSession {
    fn begin_cycle(&mut self) {
        self.start_work();    
    }

    pub fn start_work(&mut self) {
        self.pomodoro_tracker.set_work_state();
        self.clock.set_time_minutes(POMODORO_TIMER_MINUTES);
        self.countdown();
    }

    pub fn countdown(&mut self) {
        match self.pomodoro_tracker.current_state {
            PomodoroState::Working => self.countdown_work(),
            _ => (),
        }
    }

    pub fn countdown_work(&mut self) {
        loop {
            let true_elapsed: u64 = (self
                .pomodoro_tracker
                .started_at
                .unwrap()
                .elapsed()
                .as_millis()
                ) as u64;

            let clock_elapsed = (POMODORO_TIMER_MINUTES * 60000) - self.clock.get_ms_from_time();
            
            let sync_offset = true_elapsed - clock_elapsed;

            sleep(Duration::from_millis(1000 - sync_offset));

            self.clock.decrement_one_second();

            println!("Remaining time: {}", self.clock.get_time());

            if &self.clock.get_ms_from_time() == &0 {
                break;
            }
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let user_id = msg.author.id.0 as i64;

        if let Some(task_description) = msg.content.strip_prefix(TASK_ADD_COMMAND) {
            let task_description = task_description.trim();
            sqlx::query!(

                "INSERT INTO task (description, user_id) VALUES (?, ?)",
                task_description,
                user_id,
            )
            .execute(&self.database) // < Where the command will be executed
            .await
            .unwrap();

            let response = format!("Successfully added `{}` to your todo list", task_description);
            msg.channel_id.say(&ctx, response).await.unwrap();
        } else if let Some(task_index) = msg.content.strip_prefix(TASK_REMOVE_COMMAND) {
            let task_index = task_index.trim().parse::<i64>().unwrap() - 1;

            // "SELECT" will return to "entry" the rowid of the todo rows where the user_Id column = user_id.
            let entry = sqlx::query!(
                "SELECT rowid, description FROM task WHERE user_id = ? ORDER BY rowid LIMIT 1 OFFSET ?",
                user_id,
                task_index,
            )
            .fetch_one(&self.database) // < Just one data will be sent to entry
            .await
            .unwrap();

            // Every todo row with rowid column = entry.rowid will be deleted.
            sqlx::query!("DELETE FROM task WHERE rowid = ?", entry.rowid)
                .execute(&self.database)
                .await
                .unwrap();

            let response = format!("Successfully completed `{}`!", entry.description);
            msg.channel_id.say(&ctx, response).await.unwrap();
        } else if msg.content.trim() == TASK_LIST_COMMAND {
            // "SELECT" will return just the task of all rows where user_Id column = user_id in todo.
            let todos = sqlx::query!("SELECT description FROM task WHERE user_id = ? ORDER BY rowid", user_id)
                    .fetch_all(&self.database) // < All matched data will be sent to todos
                    .await
                    .unwrap();

            let mut response = format!("You have {} pending tasks:\n", todos.len());
            for (i, task) in todos.iter().enumerate() {
                writeln!(response, "{}. {}", i + 1, task.description).unwrap();
            }

            msg.channel_id.say(&ctx, response).await.unwrap();
        } else if msg.content.trim() == HELP_COMMAND {
            if let Err(why) = msg.channel_id.say(&ctx.http, help_message()).await {
                println!("Error sending message: {:?}", why);
            }
        } else if msg.content.trim() == POMODORO_START {
            let mut pomodoro = PomodoroSession {
                pomodoro_tracker: StateTracker::new(),
                clock: Clock::new()
            };
            pomodoro.start_work();
            if let Err(why) = msg.channel_id.say(&ctx.http, help_message()).await {
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
        TASK_ADD_COMMAND => execute_add_task(),
        _ => (), 
    }
}

fn execute_help() {
    println!("Help test");
}

fn execute_add_task() {
    println!("Add task test");
}

pub struct StateTracker {
    current_order: Option<i64>,
    current_state: PomodoroState,
    started_at: Option<Instant>,
}

impl StateTracker {

    pub fn new() -> StateTracker {
        StateTracker {
            current_order: None,
            current_state: PomodoroState::None,
            started_at: None,
        }
    }

    fn increment_cycle(&mut self) {
        let new_order = match self.current_order {
            Some(num) if num < 4 => Some(num + 1),
            _ => Some(1),
        };
        self.current_order = new_order;
    }
    
    pub fn set_work_state(&mut self) {
        let now = Instant::now();
        self.started_at = Some(now);

        self.current_state = PomodoroState::Working;
        self.increment_cycle();
    }

}

enum PomodoroState {
    Working,
    ShortBreak,
    LongBreak,
    None
}

pub struct Clock {
    minutes: u64,
    seconds: u64,
}

impl Clock {
    pub fn new() -> Clock {
        Clock {
            minutes: 0,
            seconds: 0,
        }
    }

    pub fn set_time_ms(&mut self, ms: u64) {
        self.minutes = (ms / (1000 * 60)) % 60;
        self.seconds = (ms / 1000) % 60;
    }

    pub fn set_time_minutes(&mut self, minutes: u64) {
        self.set_time_ms(minutes * 60000)
    }

    pub fn get_ms_from_time(&mut self) -> u64 {
        (self.minutes * 60000) + (self.seconds * 1000)
    }

    pub fn decrement_one_second(&mut self) {
        let mut time_in_ms = self.get_ms_from_time();
        time_in_ms -= 1000;
        self.set_time_ms(time_in_ms);
    }

    pub fn get_time(&self) -> String {
        format!("{:02}:{:02}", self.minutes, self.seconds)
    }
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment");

    let database = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename("database.sqlite")
                .create_if_missing(true),
        )
        .await
        .expect("Couldn't connect to database");
    
        // Run migrations, which updates the database's schema to the latest version.   
    sqlx::migrate!("./migrations").run(&database).await.expect("Couldn't run database migrations");

    let handler = Handler {
        database,
    };


    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
