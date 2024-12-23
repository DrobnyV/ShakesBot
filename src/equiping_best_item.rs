use sf_api::command::Command;
use sf_api::gamestate::items::{Item, ItemType};
use sf_api::misc::EnumMapGet;
use sf_api::SimpleSession;
use crate::functions::log_to_file;

pub struct Equip<'a> {
    session: &'a mut SimpleSession,
}

impl<'a> Equip<'a> {
    // Accepts a mutable reference to SimpleSession
    pub fn new(session: &'a mut SimpleSession) -> Self {
        Equip { session }
    }

    pub async fn equip(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut gs = self.session.send_command(Command::Update).await?;
        let eq_items = gs.character.equipment.0.clone();  // This holds the equipment items
        let backpack = gs.character.inventory.bag.clone();  // This holds the backpack items

        for (eq_slot_index, eq_item_option) in eq_items.iter().enumerate() {
            for (back_slot_index, back_item_option) in backpack.iter().enumerate() {
                if let Some(back_item) = back_item_option {
                    // Check if the equipment slot is empty
                    if eq_item_option.1.is_none() {
                        if format!("{:?}", back_item.typ) == format!("{:?}", eq_item_option.0) {
                            let command = Command::InventoryMove {
                                inventory_from: sf_api::gamestate::items::PlayerItemPlace::MainInventory,
                                inventory_from_pos: back_slot_index,
                                inventory_to: sf_api::gamestate::items::PlayerItemPlace::Equipment,
                                inventory_to_pos: eq_slot_index,
                            };
                            self.session.send_command(command).await?;
                            log_to_file(&format!("Equipped {:?} from backpack slot {} to equipment slot {:?}", back_item.typ, back_slot_index, eq_item_option.0)).await?;
                        }
                    } else if let Some(current_item) = eq_item_option.1.as_ref() {
                        match (current_item.typ, &back_item.typ) {
                            (ItemType::Weapon { .. }, ItemType::Weapon { .. }) => {
                                if is_better_item(back_item.clone(), Some(current_item.clone())).await {
                                    let command = Command::InventoryMove {
                                        inventory_from: sf_api::gamestate::items::PlayerItemPlace::MainInventory,
                                        inventory_from_pos: back_slot_index,
                                        inventory_to: sf_api::gamestate::items::PlayerItemPlace::Equipment,
                                        inventory_to_pos: eq_slot_index,
                                    };
                                    if let Err(e) = self.session.send_command(command).await {
                                        log_to_file(&format!("Failed to move item: {:?}", e)).await?;
                                    } else {
                                        log_to_file(&format!("Replaced {:?} in equipment slot {} with better {:?}", eq_item_option.0, eq_slot_index, back_item.typ)).await?;
                                    }
                                } else {
                                }
                            },
                            _ if current_item.typ == back_item.typ => {
                                // For non-weapon items, just check if types are equal
                                if is_better_item(back_item.clone(), Some(current_item.clone())).await {
                                    log_to_file(&format!("New item is better for slot {:?}", eq_item_option.0)).await?;
                                    let command = Command::InventoryMove {
                                        inventory_from: sf_api::gamestate::items::PlayerItemPlace::MainInventory,
                                        inventory_from_pos: back_slot_index,
                                        inventory_to: sf_api::gamestate::items::PlayerItemPlace::Equipment,
                                        inventory_to_pos: eq_slot_index,
                                    };
                                    if let Err(e) = self.session.send_command(command).await {
                                        log_to_file(&format!("Failed to move item: {:?}", e)).await?;
                                    } else {
                                        log_to_file(&format!("Replaced {:?} in equipment slot {} with better {:?}", eq_item_option.0, eq_slot_index, back_item.typ)).await?;
                                    }
                                } else {
                                }
                            },
                            _ => {
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn get_attribute_value(item: Item, attr_type: sf_api::command::AttributeType) -> u32 {
    *item.attributes.get(attr_type)
}

async fn is_better_item(new_item: Item, current_item: Option<Item>) -> bool {
    let strength_weight = 1;
    let constitution_weight = 4;
    let luck_weight = 2;
    let intelligence_weight = 1;
    let dexterity_weight = 5;
    let armor_weapon_weight = 6;

    // Calculate scores based on weighted attribute values
    let new_score = calculate_attribute_score(&new_item, strength_weight, constitution_weight, luck_weight, intelligence_weight, dexterity_weight)
        + new_item.armor() * armor_weapon_weight; // Armor for non-weapons

    let current_score = if let Some(ref current) = current_item {
        calculate_attribute_score(current, strength_weight, constitution_weight, luck_weight, intelligence_weight, dexterity_weight)
            + current.armor() * armor_weapon_weight
    } else {
        0
    };

    if new_item.typ.is_weapon() {
        // Extract min and max damage for weapons
        if let ItemType::Weapon { min_dmg, max_dmg } = new_item.typ {
            let new_avg_dmg = (min_dmg + max_dmg) as f64 / 2.0; // Calculate average damage
            let new_dmg_score = new_avg_dmg * armor_weapon_weight as f64; // Weight the damage

            let current_avg_dmg = if let Some(current) = current_item.as_ref() {
                if let ItemType::Weapon { min_dmg, max_dmg } = current.typ {
                    (min_dmg + max_dmg) as f64 / 2.0
                } else {
                    0.0
                }
            } else {
                0.0
            };
            let current_dmg_score = current_avg_dmg * armor_weapon_weight as f64;

            let new_total_score = new_score as f64 + new_dmg_score;
            let current_total_score = current_score as f64 + current_dmg_score;

            return new_total_score > current_total_score;
        }
    }

    // For non-weapon items

    new_score > current_score
}

fn calculate_attribute_score(item: &Item, strength_weight: u32, constitution_weight: u32, luck_weight: u32, intelligence_weight: u32, dexterity_weight: u32) -> u32 {
    (strength_weight * get_attribute_value(item.clone(), sf_api::command::AttributeType::Strength))
        + (constitution_weight * get_attribute_value(item.clone(), sf_api::command::AttributeType::Constitution))
        + (luck_weight * get_attribute_value(item.clone(), sf_api::command::AttributeType::Luck))
        + (intelligence_weight * get_attribute_value(item.clone(), sf_api::command::AttributeType::Intelligence))
        + (dexterity_weight * get_attribute_value(item.clone(), sf_api::command::AttributeType::Dexterity))
}