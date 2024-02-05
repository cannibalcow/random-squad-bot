use chrono::Utc;
use core::result::Result;
use rand::prelude::*;
use serenity::cache::Settings;
use std::fmt::Display;
use std::fs::read_to_string;
use std::str::FromStr;
use std::time::Duration;

use serenity::all::User;
use serenity::async_trait;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, Configuration, StandardFramework};
use serenity::model::channel::Message;
use serenity::prelude::*;

#[group]
#[commands(sq)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    let framework = StandardFramework::new().group(&GENERAL_GROUP);
    framework.configure(Configuration::new().prefix("!"));

    info("Booting up bot");

    // Login with a bot token from the environment
    let token = get_token(".token");
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let mut cache_settings = Settings::default();
    cache_settings.time_to_live = Duration::new(1, 0);
    cache_settings.cache_guilds = true;
    cache_settings.cache_users = true;

    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    info("Bot up and running");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        info(&format!(
            "An error occurred while running the client: {:?}",
            why
        ));
    }
}

#[command]
async fn sq(ctx: &Context, msg: &Message) -> CommandResult {
    info(&format!(
        "Author: {:?} Msg: {:?} Channel Id: {:?}",
        msg.author
            .global_name
            .clone()
            .unwrap_or(String::from("Unknown")),
        msg.content,
        msg.channel_id
    ));

    let guild = msg.guild(&ctx.cache).unwrap().clone();

    let user_id = msg.author.id;

    let channel_id = match guild.voice_states.get(&user_id) {
        Some(vc) => vc.channel_id,
        None => return Ok(()),
    };

    let users: Vec<User> = guild
        .voice_states
        .iter()
        .filter(|(_k, v)| v.channel_id == channel_id)
        .filter_map(|(k, _)| ctx.cache.user(k))
        .map(|f| f.clone())
        .collect();

    match parse_command(msg, &users) {
        Ok(v) => match v {
            Commands::SquadCommand(cmd) => {
                info(&format!("{:?}", cmd));
                let new_team = cmd.create_teams();

                msg.reply(ctx, new_team).await?
            }
            Commands::HelpCommand(txt) => msg.reply(ctx, txt).await?,
        },

        Err(e) => {
            error(format!("{:?}", e).as_str());
            msg.reply(
                ctx,
                "Jag fattar inte vad du skriver eller så är jag dum i huvudet. Försök igen.",
            )
            .await?
        }
    };

    Ok(())
}

#[derive(Debug)]
enum ParseError {
    InvalidCommand,
    InvalidTeamSetup(String),
}

#[derive(Debug)]
enum TeamSetup {
    Duo,
    Trio,
    Squad,
}

enum Commands {
    SquadCommand(SquadCmd),
    HelpCommand(String),
}

#[derive(Debug)]
struct SquadCmd {
    team_setup: TeamSetup,
    users: Vec<String>,
}

impl Display for SquadCmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Team Setup: {:?} Users: {:?}",
            self.team_setup, self.users
        )
    }
}

impl SquadCmd {
    fn create_teams(&self) -> String {
        let mut rng = thread_rng();
        let mut teams = self.users.clone();
        teams.shuffle(&mut rng);

        let mut msg = String::new();

        let team_size = match self.team_setup {
            TeamSetup::Duo => {
                msg.push_str("-- **Duos** --\n");
                2
            }
            TeamSetup::Trio => {
                msg.push_str("-- **Trios** --\n");
                3
            }
            TeamSetup::Squad => {
                msg.push_str("-- **Squads** --\n");
                4
            }
        };

        for (i, team) in teams.chunks(team_size).enumerate() {
            msg.push_str(format!("**{}**. {}\n", i + 1, team.join(", ")).as_str())
        }

        msg
    }
}

impl FromStr for TeamSetup {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return match s.to_lowercase().as_str() {
            "duo" => Ok(TeamSetup::Duo),
            "trio" => Ok(TeamSetup::Trio),
            "squad" => Ok(TeamSetup::Squad),
            e => Err(ParseError::InvalidTeamSetup(format!(
                "Invalid team setup {}",
                e
            ))),
        };
    }
}

fn parse_command(msg: &Message, users: &[User]) -> Result<Commands, ParseError> {
    let args: Vec<String> = msg.content.split(' ').map(|f| f.to_string()).collect();

    if args.len() == 1 {
        let help_text = String::from("I will fetch all users in voice chat and randomize teams.\n !sq <duo|trio|squad> !<username to exclude>\nExclude nicks are lowercase.");
        return Ok(Commands::HelpCommand(help_text));
    }

    let team_setup = match args.get(1) {
        Some(teamsetup) => TeamSetup::from_str(teamsetup)?,
        None => return Err(ParseError::InvalidCommand),
    };

    let exclude: Vec<String> = args
        .iter()
        .skip(1)
        .filter(|f| f.to_string().starts_with('!'))
        .map(|user| user.chars().skip(1).collect())
        .collect();

    info(&format!("Exludes users: {:?}", exclude));

    let all_users = users
        .iter()
        .map(|u| u.name.to_string())
        .collect::<Vec<String>>();

    info(&format!("Voice users: {:?}", all_users));

    let mut users: Vec<String> = all_users
        .iter()
        .filter(|user| !exclude.contains(user))
        .cloned()
        .collect();

    let mut extra_users: Vec<String> = args
        .iter()
        .skip(2)
        .filter(|f| !f.starts_with('!'))
        .map(|f| f.to_string())
        .collect();

    info(&format!("Extra users: {:?}", extra_users));

    users.append(&mut extra_users);

    let cmd = SquadCmd { team_setup, users };

    Ok(Commands::SquadCommand(cmd))
}

fn get_token(path: &str) -> String {
    read_to_string(path).unwrap()
}

fn info(msg: &str) {
    let now = Utc::now();
    println!("| {} | INFO | {}", now, msg)
}

fn error(msg: &str) {
    let now = Utc::now();
    println!("| {} | ERROR | {}", now, msg)
}
