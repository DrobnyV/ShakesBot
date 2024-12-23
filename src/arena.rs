use std::time::Duration;
use sf_api::command::Command;
use sf_api::command::Command::Update;
use sf_api::SimpleSession;
use crate::functions::{log_to_file, time_remaining};

pub struct Arena<'a> {
    session: &'a mut SimpleSession,
}

impl<'a> Arena<'a> {
    // Accepts a mutable reference to SimpleSession
    pub fn new(session: &'a mut SimpleSession) -> Self {
        Arena { session }
    }

    async fn find_weakest_player(&mut self) -> Option<String> {
        let gs = match self.session.send_command(Command::Update).await {
            Ok(gs) => gs,
            Err(_) => return None,
        };
        let a = gs.arena.enemy_ids.clone();
        let mut lowest_enemy_attributes = u32::MAX;
        let mut lowest_enemy_name = None;
        let mut is_first_enemy = true;

        for enemy in a {
            if let Ok(_) = self.session.send_command(Command::ViewPlayer { ident: enemy.to_string() }).await {
                if let Some(player) = self.session.game_state().unwrap().lookup.lookup_pid(enemy) {
                    let enemy_attributes = player.base_attributes.values().sum::<u32>() + player.bonus_attributes.values().sum::<u32>()
                        + ((player.min_damage_base + player.max_damage_base)/2);
                    if is_first_enemy || enemy_attributes < lowest_enemy_attributes {
                        lowest_enemy_attributes = enemy_attributes;
                        lowest_enemy_name = Some(player.name.clone());
                        is_first_enemy = false;
                    }
                }
            }
        }

        lowest_enemy_name
    }

    pub async fn fight_arena(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let gs = self.session.send_command(Update).await?;
        let rem = time_remaining(gs.arena.next_free_fight.unwrap());

        if rem <= Duration::from_secs(0) {
            if let Some(weakest_player) = self.find_weakest_player().await {
                self.session.send_command(Command::Fight {
                    name: weakest_player,
                    use_mushroom: false,
                }).await?;
                if let Some(game_state) = self.session.game_state() {
                    log_to_file(&format!("Result of a fight {:?}", game_state.last_fight.clone().unwrap().has_player_won)).await?;
                } else {
                    eprintln!("Failed to get game state.");
                }
            } else {
                eprintln!("No weakest player found.");
            }
        }else{
            log_to_file(&format!("{:?} minutes until the next arena fight is available", rem/60)).await?;
        }

        Ok(())
    }
}