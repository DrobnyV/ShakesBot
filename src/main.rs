mod quest;
#[cfg(test)]
mod testing;
mod equiping_best_item;

use std::time::Duration;
use sf_api::{SimpleSession};
use tokio;
use tokio::time::sleep;
use crate::quest::Questing;
use std::io::{self, Write};      // For reading input from the user
use rpassword::read_password;
use crate::equiping_best_item::Equip;

#[tokio::main]
async fn main() {
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
    let mut sessions = SimpleSession::login_sf_account(username, &password)
        .await
        .expect("Login failed");
    println!("Logged in successfully. Starting");
    loop {
        for session in &mut sessions {
            let mut equip = Equip::new(session);
            equip.equip().await;
            let mut quest = Questing::new(session);
            quest.questing().await;
            break
        }
        sleep(Duration::from_secs(60)).await;
    }


}