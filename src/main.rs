mod quest;
#[cfg(test)]
mod testing;
mod equiping_best_item;
mod dungeon;

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

fn setup_logger() -> Result<(), fern::InitError> {
    Dispatch::new()
        // Write to log file
        .chain(fern::log_file("application.log")?)
        // Optionally write to console as well
        .chain(io::stdout())
        // Use a custom log format
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                message
            ))
        })
        // Set log level (e.g., Info and above)
        .level(log::LevelFilter::Info)
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
    info!("Starting main loop...");
    loop {
        // Attempt to log in with the provided credentials
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
            let mut equip = Equip::new(session);
            if let Err(e) = equip.equip().await {
                error!("Failed to equip items: {:?}", e);
            }

            let mut quest = Questing::new(session);
            if let Err(e) = quest.questing().await {
                error!("Questing failed: {:?}", e);
            }

            // Break after processing one session for simplicity
            break;
        }
        sleep(Duration::from_secs(60)).await;
    }
}
