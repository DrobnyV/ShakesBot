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
use sf_api::gamestate::rewards::Event::{EpicQuestExtravaganza, ExceptionalXPEvent, OneBeerTwoBeerFreeBeer};
use sf_api::gamestate::rewards::{Reward, RewardType};
use sf_api::gamestate::tavern::CurrentAction::Expedition;
use sf_api::gamestate::tavern::{ExpeditionEncounter, ExpeditionStage, ExpeditionThing};
use crate::functions::{log_to_file, sell_the_worst_item, time_remaining};

pub struct Exping<'a> {
    session: &'a mut SimpleSession,
}

impl<'a> Exping<'a> {
    // Accepts a mutable reference to SimpleSession
    pub fn new(session: &'a mut SimpleSession) -> Self {
        Exping { session }
    }

    pub async fn Exping(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let gs = self.session.send_command(Command::Update).await?;
            let exp = &gs.tavern.expeditions;

            let Some(active) = exp.active() else {
                // We do not currently have an expedition running. Make sure we are
                // idle
                if !gs.tavern.is_idle() {
                    log_to_file("Waiting/Collection other actions is not part of this example").await?;
                    break;
                }

                let expeditions = match gs.tavern.available_tasks() {
                    AvailableTasks::Quests(_) => {
                        // We can only do quest, let's figure out why. Note that
                        // normally you could just do quests here
                        if !exp.is_event_ongoing() {
                            log_to_file("Expeditions are currently not enabled, so we can not do anything").await?;
                            break;
                        }
                        if gs.tavern.questing_preference == ExpeditionSetting::PreferQuests {
                            // This means we could do expeditions, but they are
                            // disabled in the settings
                            if !gs.tavern.can_change_questing_preference() {
                                log_to_file("Expeditions are disabled in the settings and that setting can not be changed today").await?;
                                break;
                            }
                            log_to_file("Changing expedition setting").await?;
                            self.session
                                .send_command(
                                    Command::SetQuestsInsteadOfExpeditions {
                                        value: ExpeditionSetting::PreferExpeditions,
                                    },
                                )
                                .await?;
                            continue;
                        }
                        log_to_file("There seem to be no expeditions").await?;
                        break;
                    }
                    AvailableTasks::Expeditions(expeditions) => expeditions,
                };

                // We would normally have to choose which expedition is the best.
                // For now we just choose the first one though
                let target = expeditions.first().unwrap();

                // Make sure we have enough thirst for adventure to do the
                // expeditions
                if target.thirst_for_adventure_sec > gs.tavern.thirst_for_adventure_sec {
                    let has_extra_beer = gs.character.equipment.has_enchantment(Enchantment::ThirstyWanderer);
                    let events = gs.specials.events.active.clone();
                    let _ = gs;
                    let mut a = false;
                    for event in events {
                        if event == ExceptionalXPEvent || event == EpicQuestExtravaganza || event == OneBeerTwoBeerFreeBeer {
                            let gs = self.session.send_command(Command::Update).await?;
                            if gs.character.mushrooms > 0 && gs.tavern.beer_drunk < (10 + has_extra_beer as u8) {
                                log_to_file("Buying beer").await?;
                                self.session.send_command(Command::BuyBeer).await?;
                                a = true;
                                continue;
                            } else {
                                log_to_file("Starting city guard").await?;
                                self.session.send_command(Command::StartWork { hours: 10 }).await?;
                                break;
                            }
                        }
                    }
                    if !a {
                        let gs = self.session.send_command(Command::Update).await?;
                        if gs.character.mushrooms > 0 && gs.tavern.beer_drunk < (0 + has_extra_beer as u8) {
                            log_to_file("Buying beer").await?;
                            self.session.send_command(Command::BuyBeer).await?;
                            continue;
                        } else {
                            log_to_file("Starting city guard").await?;
                            self.session.send_command(Command::StartWork { hours: 10 }).await?;
                            break;
                        }
                    } else {
                        a = false;
                    }
                }

                // We should be all good to start the expedition
                log_to_file("Starting expedition").await?;
                self.session
                    .send_command(Command::ExpeditionStart { pos: 0 })
                    .await?;
                continue;
            };
            let current = active.current_stage();

            let cmd = match current {
                ExpeditionStage::Boss(_) => {
                    log_to_file("Fighting the expedition boss").await?;
                    Command::ExpeditionContinue
                }
                ExpeditionStage::Rewards(rewards) => {
                    if rewards.is_empty() {
                        log_to_file("No rewards to choose from").await?;
                        continue; // Changed from panic to continue with logging
                    }
                    log_to_file("Picking reward").await?;
                    let priority_order = [
                        RewardType::LuckyCoins,
                        RewardType::Mushrooms,
                        RewardType::Stone,
                        RewardType::Wood,
                        RewardType::QuicksandGlass,
                        RewardType::Silver,
                    ];

                    let selected_reward = rewards.iter().enumerate()
                        .max_by(|(_, a), (_, b)| {
                            let a_priority = priority_order.iter().position(|&r| r == a.typ).unwrap_or(priority_order.len());
                            let b_priority = priority_order.iter().position(|&r| r == b.typ).unwrap_or(priority_order.len());
                            b_priority.cmp(&a_priority) // Reverse cmp because max_by gives us the highest priority (which is the lowest index)
                        });
                    match selected_reward {
                        Some((index, _)) => {
                            log_to_file(&format!("Selected reward at index: {}", index)).await?;
                            Command::ExpeditionPickReward {pos: index}
                        },
                        None => {
                            // If no reward matches our priority list, you might want to pick the first one or handle this scenario differently
                            log_to_file("No suitable reward found; picking first available").await?;
                            Command::ExpeditionPickReward {pos: 0}
                        }
                    }
                }
                ExpeditionStage::Encounters(roads) => {
                    if roads.is_empty() {
                        log_to_file("No crossroads to choose from").await?;
                        continue; // Handle this scenario without panicking
                    }
                    let active_expedition = gs.tavern.expeditions.active().expect("No active expedition found");
                    let target_thing = &active_expedition.target_thing;
                    const MAX_PRIORITY_LENGTH: usize = 6; // Adjust based on the longest priority list

                    // Define a priority or logic for which encounter to choose
                    let priority_order: [ExpeditionThing; MAX_PRIORITY_LENGTH] = match target_thing {
                        ExpeditionThing::ToiletPaper => [
                            ExpeditionThing::ToiletPaperBounty,
                            ExpeditionThing::ToiletPaper,
                            ExpeditionThing::DummyBounty,
                            ExpeditionThing::Unknown, // Default to fill the array
                            ExpeditionThing::Unknown,
                            ExpeditionThing::Unknown,
                        ],
                        ExpeditionThing::Dragon => [
                            ExpeditionThing::DragonBounty,
                            ExpeditionThing::Dragon,
                            ExpeditionThing::Bait,
                            ExpeditionThing::DummyBounty,
                            ExpeditionThing::Unknown,
                            ExpeditionThing::Unknown,
                        ],
                        ExpeditionThing::Cake => [
                            ExpeditionThing::Cake,
                            ExpeditionThing::DummyBounty,
                            ExpeditionThing::Unknown,
                            ExpeditionThing::Unknown,
                            ExpeditionThing::Unknown,
                            ExpeditionThing::Unknown,
                        ],
                        ExpeditionThing::RoyalFrog => [
                            ExpeditionThing::FrogBounty,
                            ExpeditionThing::RoyalFrog,
                            ExpeditionThing::Prince,
                            ExpeditionThing::DummyBounty,
                            ExpeditionThing::Unknown,
                            ExpeditionThing::Unknown,
                        ],
                        ExpeditionThing::BurntCampfire => [
                            ExpeditionThing::BurntCampfireBounty,
                            ExpeditionThing::BurntCampfire,
                            ExpeditionThing::CampFire,
                            ExpeditionThing::Phoenix,
                            ExpeditionThing::DummyBounty,
                            ExpeditionThing::Unknown,
                        ],
                        ExpeditionThing::WinnersPodium => [
                            ExpeditionThing::WinnerPodiumBounty,
                            ExpeditionThing::WinnersPodium,
                            ExpeditionThing::SmallHurdle,
                            ExpeditionThing::BigHurdle,
                            ExpeditionThing::DummyBounty,
                            ExpeditionThing::Unknown,
                        ],
                        ExpeditionThing::BrokenSword => [
                            ExpeditionThing::BrokenSwordBounty,
                            ExpeditionThing::BrokenSword,
                            ExpeditionThing::BentSword,
                            ExpeditionThing::SwordInStone,
                            ExpeditionThing::DummyBounty,
                            ExpeditionThing::Unknown,
                        ],
                        ExpeditionThing::Klaus => [
                            ExpeditionThing::KlausBounty,
                            ExpeditionThing::Klaus,
                            ExpeditionThing::Body,
                            ExpeditionThing::Feet,
                            ExpeditionThing::Hand,
                            ExpeditionThing::DummyBounty,
                        ],
                        ExpeditionThing::Unicorn => [
                            ExpeditionThing::UnicornBounty,
                            ExpeditionThing::Unicorn,
                            ExpeditionThing::Rainbow,
                            ExpeditionThing::Donkey,
                            ExpeditionThing::UnicornHorn,
                            ExpeditionThing::DummyBounty,
                        ],
                        ExpeditionThing::Balloons => [
                            ExpeditionThing::BaloonBounty,
                            ExpeditionThing::Balloons,
                            ExpeditionThing::Well,
                            ExpeditionThing::Girl,
                            ExpeditionThing::DummyBounty,
                            ExpeditionThing::Unknown,
                        ],
                        ExpeditionThing::RevealingCouple => [
                            ExpeditionThing::RevealingCoupleBounty,
                            ExpeditionThing::RevealingCouple,
                            ExpeditionThing::Socks,
                            ExpeditionThing::ClothPile,
                            ExpeditionThing::DummyBounty,
                            ExpeditionThing::Unknown,
                        ],
                        _ => [
                            ExpeditionThing::DummyBounty,
                            ExpeditionThing::Unknown,
                            ExpeditionThing::Unknown,
                            ExpeditionThing::Unknown,
                            ExpeditionThing::Unknown,
                            ExpeditionThing::Unknown,
                        ],
                    };

                    // Now choose the best road based on priority
                    let best_road_index = roads.iter().enumerate().max_by(|(_, a), (_, b)| {
                        let a_priority = priority_order.iter().position(|&r| r == a.typ).unwrap_or(priority_order.len());
                        let b_priority = priority_order.iter().position(|&r| r == b.typ).unwrap_or(priority_order.len());
                        if a_priority == priority_order.len() && b_priority == priority_order.len() {
                            // If neither matches priority, compare by heroism
                            b.heroism.cmp(&a.heroism)
                        } else {
                            b_priority.cmp(&a_priority)
                        }
                    }).map(|(index, _)| index).unwrap_or(0);

                    log_to_file(&format!("Choosing crossroad at index: {}", best_road_index)).await?;

                    Command::ExpeditionPickEncounter { pos: best_road_index }
                }
                ExpeditionStage::Finished => {
                    // Between calling current_stage and now the expedition
                    // finished. next time we call active, it will be None
                    continue;
                }
                ExpeditionStage::Waiting(until) => {
                    let remaining = time_remaining(until);
                    if remaining.as_secs() > 60 && gs.tavern.quicksand_glasses > 0 {
                        log_to_file(&format!("Skipping the {}s wait", remaining.as_secs())).await?;
                        Command::ExpeditionSkipWait {
                            typ: TimeSkip::Glass,
                        }
                    } else {
                        log_to_file(&format!("Waiting {}s until next expedition step", remaining.as_secs())).await?;
                        sleep(remaining).await;
                        Command::Update
                    }
                }
                ExpeditionStage::Unknown => {
                    log_to_file("Unknown expedition stage encountered").await?;
                    continue; // Changed
                },
            };
            sleep(Duration::from_secs(1)).await;
            self.session.send_command(cmd).await.unwrap();
        }

        Ok(())
    }


}

