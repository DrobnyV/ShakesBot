use std::fs::OpenOptions;
use chrono::Local;
use sf_api::command::Command;
use sf_api::gamestate::items::{Item};
use sf_api::misc::EnumMapGet;
use sf_api::SimpleSession;
use std::io::Write;
pub struct Equip<'a> {
    session: &'a mut SimpleSession,
}
impl<'a> Equip<'a> {
    // Accepts a mutable reference to SimpleSession
    pub fn new(session: &'a mut SimpleSession) -> Self {
        Equip { session }
    }

    pub async fn equip(&mut self) {
        let mut gs = self.session.send_command(Command::Update).await.unwrap();
        let eq_items = gs.character.equipment.0.clone();  // This holds the equipment items
        let backpack = gs.character.inventory.bag.clone();  // This holds the backpack items

        for (eq_slot_index, eq_item_option) in eq_items.iter().enumerate() {
            for (back_slot_index, back_item_option) in backpack.iter().enumerate() {
                if let Some(back_item) = back_item_option {
                    // Check if the equipment slot is empty
                    if eq_item_option.1.is_none() {
                        // Equip backpack item to empty equipment slot
                        if format!("{:?}", back_item.typ) == format!("{:?}", eq_item_option.0) {
                            let command = Command::InventoryMove {
                                inventory_from: sf_api::gamestate::items::PlayerItemPlace::MainInventory,  // The item is coming from the backpack
                                inventory_from_pos: back_slot_index,       // Position in the backpack
                                inventory_to: sf_api::gamestate::items::PlayerItemPlace::Equipment,  // Moving to equipment
                                inventory_to_pos: eq_slot_index,          // Position in the equipment slot
                            };

                            // Send the command asynchronously
                            self.session.send_command(command).await.unwrap();
                            log_to_file(&format!("Equipped {:?} from backpack slot {} to equipment slot {:?}", back_item.typ, back_slot_index, eq_item_option.0)).await;
                        }
                    } else if eq_item_option.1.clone().unwrap().typ == back_item.typ {
                        // Equip backpack item to equipment slot if it's better
                        if is_better_item(back_item.clone(),eq_item_option.1.clone()) {
                            let command = Command::InventoryMove {
                                inventory_from: sf_api::gamestate::items::PlayerItemPlace::MainInventory,
                                inventory_from_pos: back_slot_index,
                                inventory_to: sf_api::gamestate::items::PlayerItemPlace::Equipment,
                                inventory_to_pos: eq_slot_index,
                            };

                            // Send the command asynchronously
                            self.session.send_command(command).await.unwrap();
                            log_to_file(&format!("Replaced {:?} in equipment slot {} with better {:?}", eq_item_option.0, eq_slot_index, back_item.typ)).await;
                        }
                    }
                }
            }
        }
    }

}
fn get_attribute_value(item: Item, attr_type: sf_api::command::AttributeType) -> u32 {
    *item.attributes.get(attr_type)
}
fn is_better_item(new_item: Item, current_item: Option<Item>) -> bool {
    // Assign weights to each attribute for priority comparison
    let strength_weight = 5;
    let constitution_weight =4;
    let luck_weight = 2;

    // Calculate scores based on weighted attribute values
    let new_score = (strength_weight * get_attribute_value(new_item.clone(), sf_api::command::AttributeType::Strength))
        + (constitution_weight * get_attribute_value(new_item.clone(), sf_api::command::AttributeType::Constitution))
        + (luck_weight * get_attribute_value(new_item.clone(), sf_api::command::AttributeType::Luck))
        + get_attribute_value(new_item.clone(), sf_api::command::AttributeType::Intelligence)
        + get_attribute_value(new_item.clone(), sf_api::command::AttributeType::Dexterity);

    let current_score = (strength_weight * get_attribute_value(current_item.clone().unwrap(), sf_api::command::AttributeType::Strength))
        + (constitution_weight * get_attribute_value(current_item.clone().unwrap(), sf_api::command::AttributeType::Constitution))
        + (luck_weight * get_attribute_value(current_item.clone().unwrap(), sf_api::command::AttributeType::Luck))
        + get_attribute_value(current_item.clone().unwrap(), sf_api::command::AttributeType::Intelligence)
        + get_attribute_value(current_item.clone().unwrap(), sf_api::command::AttributeType::Dexterity);

    // A higher new_score means the new_item is better
    new_score > current_score
}
async fn log_to_file(message: &str) {
    let now = Local::now();
    let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("help.log")
        .expect("Unable to open or create help.log file");

    writeln!(file, "[{}] {}", timestamp, message).expect("Unable to write to help.log file");
}