#![allow(unused)]
use std::{borrow::Borrow, time::Duration};

use chrono::{DateTime, Local, Timelike};
use sf_api::{
    command::{Command, ExpeditionSetting, TimeSkip},
    gamestate::{
        items::{Enchantment, EquipmentSlot},
        tavern::{AvailableTasks, CurrentAction},
    },
    misc::EnumMapGet,
    SimpleSession,
};
use sf_api::gamestate::GameState;
use tokio::time::sleep;
use std::fs::OpenOptions;
use std::io::Write;
use sf_api::gamestate::items::PlayerItemPlace;
use crate::functions::{log_to_file, sell_the_worst_item, time_remaining};

pub struct Questing<'a> {
    session: &'a mut SimpleSession,
}
impl<'a> Questing<'a> {
    // Accepts a mutable reference to SimpleSession
    pub fn new(session: &'a mut SimpleSession) -> Self {
        Questing { session }
    }



    pub async fn questing(&mut self) -> Result<(), Box<dyn std::error::Error>> {




        loop {
            sleep(Duration::from_secs(2)).await;
            let gs = self.session.send_command(Command::Update).await.unwrap();
            let current_time = Local::now();
            let end_hour = 22;
            let remaining_hours = end_hour  - current_time.hour() as i32;
            let mut rem_help = remaining_hours;
            if remaining_hours > 11 || remaining_hours < 2{
                rem_help = 11;
            }
            match &gs.tavern.current_action {
                CurrentAction::Idle => match gs.tavern.available_tasks() {
                    AvailableTasks::Quests(q) => {
                        if remaining_hours > 1 {
                            log_to_file("Starting city guard to work until 22:00").await?;
                            self.session
                                .send_command(Command::StartWork { hours: rem_help as u8 - 1 })
                                .await?;

                            break;
                        }
                        let mut best_quest_index = 0;
                        let mut best_quest = gs.tavern.quests.first().unwrap().clone();
                        for (index, quest) in gs.tavern.quests.clone().iter().enumerate() {
                            if quest.base_experience > best_quest.base_experience{
                                best_quest = quest.clone();
                                best_quest_index = index;
                            }
                        }

                        if best_quest.base_length > gs.tavern.thirst_for_adventure_sec {
                            let has_extra_beer = gs
                                .character
                                .equipment
                                .has_enchantment(Enchantment::ThirstyWanderer);

                            if gs.character.mushrooms > 0 && gs.tavern.beer_drunk < (0 + has_extra_beer as u8) {
                                log_to_file("Buying beer").await?;
                                self.session
                                    .send_command(Command::BuyBeer)
                                    .await
                                    .unwrap();
                                continue;
                            } else {
                                log_to_file("Starting city guard").await?;
                                self.session
                                    .send_command(Command::StartWork { hours: rem_help as u8 - 1 })
                                    .await?;
                                break;
                            }
                        }
                        log_to_file("Starting the next quest").await?;

                        if best_quest.item.is_some() && gs.character.inventory.free_slot().is_none() {
                            sell_the_worst_item(self.session);
                        }

                        let q = self.session
                            .send_command(Command::StartQuest {
                                quest_pos: best_quest_index,
                                overwrite_inv: true,
                            })
                            .await?;
                        continue;
                    }
                    AvailableTasks::Expeditions(_) => {
                        if !gs.tavern.can_change_questing_preference() {
                            log_to_file("We cannot do quests because we have done expeditions today already").await?;
                            break;
                        }
                        log_to_file("Changing questing setting").await?;
                        self.session
                            .send_command(Command::SetQuestsInsteadOfExpeditions {
                                value: ExpeditionSetting::PreferQuests,
                            })
                            .await?;
                        continue;
                    }
                },
                CurrentAction::Quest {
                    quest_idx,
                    busy_until,
                } => {
                    let remaining = time_remaining(busy_until);
                    let mut skip = None;

                    if remaining > Duration::from_secs(60) {
                        if gs.tavern.quicksand_glasses > 0 {
                            skip = Some(TimeSkip::Glass);
                        } else if gs.character.mushrooms > 0 && gs.tavern.mushroom_skip_allowed {
                            skip = Some(TimeSkip::Mushroom);
                        }
                    }
                    if let Some(skip) = skip {
                        log_to_file(&format!("Skipping the remaining {:?} with a {:?}", remaining, skip)).await?;
                        self.session
                            .send_command(Command::FinishQuest { skip: Some(skip) })
                            .await
                            .unwrap();
                    } else {
                        log_to_file(&format!("Waiting {:?} until the quest is finished", remaining)).await?;
                        sleep(remaining).await;
                        self.session
                            .send_command(Command::FinishQuest { skip })
                            .await?;
                    }
                }
                CurrentAction::CityGuard { hours, busy_until } => {
                    let remaining = time_remaining(busy_until);
                    if remaining_hours <= 1 || remaining == Duration::ZERO{
                        log_to_file("Waiting for finishing the city guard job").await?;
                        sleep(time_remaining(busy_until)).await;
                        self.session.send_command(Command::FinishWork).await;
                        log_to_file("Worked finished").await?;
                    } else {
                        let rem_help = remaining / 60;
                        log_to_file(&format!("{:?} minutes until the city guard is finished", rem_help)).await?;
                        break;
                    }
                    continue;
                }
                _ => {
                    log_to_file("Expeditions are not part of this example").await?;
                    break;
                }
            }
        }


        Ok(())
    }

}

