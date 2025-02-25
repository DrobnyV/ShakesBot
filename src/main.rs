mod quest;
#[cfg(test)]
mod testing;
mod equiping_best_item;
mod dungeon;
mod functions;
mod arena;
mod Expedision;

use std::time::Duration;
use sf_api::SimpleSession;
use tokio;
use tokio::time::sleep;
use crate::quest::Questing;
use std::io::{self, Write}; // For reading input from the user
use rpassword::read_password;
use sf_api::command::Command;
use crate::equiping_best_item::Equip;
use log::{error, info, warn};
use chrono::Local;
use fern::Dispatch;
use sf_api::gamestate::tavern::CurrentAction::Expedition;
use crate::arena::Arena;
use crate::dungeon::Dungeons;
use crate::Expedision::Exping;
use crate::functions::{log_to_file, sell_the_worst_item};

fn setup_logger() -> Result<(), fern::InitError> {
    // Create a dispatch for logging
    Dispatch::new()
        // Log to a file named `application.log`
        .chain(fern::log_file("application.log")?)
        // Also log to the console (stdout)
        .chain(io::stdout())
        // Use a custom log format for both outputs
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"), // Timestamp
                record.level(),                           // Log level
                message                                   // Log message
            ))
        })
        // Set the default log level (Info and above)
        .level(log::LevelFilter::Info)
        // Apply the logger
        .apply()?;
    Ok(())
}

#[tokio::main]
async fn main() {
    // Initialize logging
    setup_logger().expect("Failed to initialize logger");

    // Prompt the user for the username
    print!("Enter your username: ");
    io::stdout().flush().unwrap(); // Ensures the prompt appears immediately
    let mut username = String::new();
    io::stdin().read_line(&mut username).unwrap();
    let username = username.trim(); // Removes any trailing newline

    // Prompt the user for the password
    print!("Enter your password: ");
    io::stdout().flush().unwrap(); // Ensures the password prompt is displayed
    let password = read_password().expect("Failed to read password");
    // Attempt to log in with the provided credentials

    info!("Starting main loop...");
    loop {
        let mut sessions = match SimpleSession::login_sf_account(username, &password).await {
            Ok(s) => {
                info!("Logged in successfully.");
                s
            }
            Err(e) => {
                error!("Login failed: {:?}", e);
                return;
            }
        };

        for session in &mut sessions {
            session.send_command(Command::Update).await.expect("Failed to update");
            log_to_file(&*session.game_state_mut().unwrap().character.name.clone()).await.expect("Failed to log to file");
            log_to_file("{").await.expect("Failed to log to file");

            if  session.game_state().unwrap().character.inventory.free_slot().is_none(){
                sell_the_worst_item(session).await.expect("Cannot sell item");
            }
            for item in session.game_state().unwrap().character.inventory.bag.clone(){

            }


            let mut equip = Equip::new(session);
            if let Err(e) = equip.equip().await {
                error!("Failed to equip items: {:?}", e);
            }
            /*
            let mut quest = Questing::new(session);
            if let Err(e) = quest.questing().await {
                error!("Questing failed: {:?}", e);
            }
            */
            let mut exp =Exping::new(session);
            if let Err(e) = exp.Exping().await {
                error!("Failed to start expedition: {:?}", e);
            }
            let mut dungeon = Dungeons::new(session);
            if let Err(e) = dungeon.do_dungeons().await {
                error!("Dungeon failed: {:?}", e);
            }

            let mut arena = Arena::new(session);
            if let Err(e) = arena.fight_arena().await {
                error!("Dungeon failed: {:?}", e);
            }

            log_to_file("}").await.expect("Failed to log to file");
            break;
        }
        sleep(Duration::from_secs(60)).await;

    }
}
