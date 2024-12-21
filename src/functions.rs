use std::borrow::Borrow;
use std::fs::OpenOptions;
use std::time::Duration;
use chrono::{DateTime, Local};
use sf_api::command::Command;
use sf_api::gamestate::items::PlayerItemPlace;
use sf_api::SimpleSession;
use std::io::Write;

pub async fn log_to_file(message: &str) -> Result<(), Box<dyn std::error::Error>> {
    let now = Local::now();
    let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();

    // Open or create the file in append mode
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("help.log")?;

    // Write to the file
    writeln!(file, "[{}] {}", timestamp, message)?;

    // Also print to the console
    println!("[{}] {}", timestamp, message);

    Ok(())
}
pub fn time_remaining<T: Borrow<DateTime<Local>>>(time: T) -> Duration {
    (*time.borrow() - Local::now()).to_std().unwrap_or_default()
}
pub async fn sell_the_worst_item(mut session: &mut SimpleSession) -> Result<(), Box<dyn std::error::Error>> {
    let gs = session.send_command(Command::Update).await?;
    let backpack = gs.character.inventory.bag.clone();
    let mut bad_item_index = 0;
    let mut bad_valu = 999999999;
    for (back_slot_index, back_item_option) in backpack.iter().enumerate() {
        if !back_item_option.is_none(){
            if bad_valu > back_item_option.clone().unwrap().price{
                bad_item_index = back_slot_index;
                bad_valu = back_item_option.clone().unwrap().price;
            }
        }
    }
    session.send_command(Command::SellShop { inventory: PlayerItemPlace::MainInventory, inventory_pos: bad_item_index }).await?;
    log_to_file(&format!("Sold an item on index {:?}", bad_item_index)).await?;
    Ok(())
}