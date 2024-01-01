use core::result::Result;
use rand::prelude::*;
use std::fs::read_to_string;
use std::str::FromStr;

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
    framework.configure(Configuration::new().prefix("!")); // set the bot's prefix to "~"

    // Login with a bot token from the environment
    let token = get_token(".token");
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn sq(ctx: &Context, msg: &Message) -> CommandResult {
    println!("MSG: {:?} Channel Id: {:?}", msg.content, msg.channel_id);

    let x = msg.guild(&ctx.cache).unwrap().clone();

    let user_id = msg.author.id;

    let channel_id = match x.voice_states.get(&user_id) {
        Some(vc) => vc.channel_id,
        None => return Ok(()),
    };

    let users: Vec<User> = x
        .voice_states
        .iter()
        .filter(|(_k, v)| v.channel_id == channel_id)
        .filter_map(|(k, _)| ctx.cache.user(k))
        .map(|f| f.clone())
        .collect();

    for u in &users {
        println!("User: {}", u.name);
    }

    println!("========================================");

    match parse_command(&msg, &users) {
        Ok(v) => match v {
            Commands::SquadCommand(cmd) => msg.reply(ctx, cmd.create_teams()).await?,
            Commands::HelpCommand(txt) => msg.reply(ctx, txt).await?,
        },

        Err(e) => {
            eprintln!("ERROR: {:?}", e);
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

impl SquadCmd {
    fn create_teams(&self) -> String {
        let mut rng = thread_rng();
        let mut teams = self.users.clone();
        teams.shuffle(&mut rng);

        let mut msg = String::new();

        let team_size = match self.team_setup {
            TeamSetup::Duo => {
                msg.push_str("-- Duos --\n");
                2
            }
            TeamSetup::Trio => {
                msg.push_str("-- Trios --\n");
                3
            }
            TeamSetup::Squad => {
                msg.push_str("-- Squads --\n");
                4
            }
        };

        for (i, team) in teams.chunks(team_size).enumerate() {
            msg.push_str(format!("{}. {}\n", i + 1, team.join(", ")).as_str())
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

fn parse_command(msg: &Message, users: &Vec<User>) -> Result<Commands, ParseError> {
    let args: Vec<String> = msg.content.split(" ").map(|f| f.to_string()).collect();

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
        .filter(|f| f.to_string().starts_with("!") && f.to_string() != "!sq")
        .map(|f| f.clone())
        .map(|user| user.chars().skip(1).collect())
        .collect();

    println!("Exludes users: {:?}", exclude);

    let all_users = users
        .iter()
        .map(|u| u.name.to_string())
        .collect::<Vec<String>>();

    let users = all_users
        .iter()
        .filter(|user| !exclude.contains(user))
        .cloned()
        .collect();

    let cmd = SquadCmd { team_setup, users };

    println!("Parsed: ");
    println!("{:?}", cmd);

    return Ok(Commands::SquadCommand(cmd));
}

fn get_token(path: &str) -> String {
    return read_to_string(path).unwrap();
}
