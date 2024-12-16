use strum::IntoEnumIterator;
use std::borrow::Borrow;
use std::time::Duration;

use sf_api::command::Command;
use sf_api::gamestate::dungeons::{Dungeon, LightDungeon};
use sf_api::SimpleSession;
use sf_api::simulate::Monster;
use tokio::time::sleep;
use crate::functions::{log_to_file, sell_the_worst_item, time_remaining};

pub struct Dungeons<'a> {
    session: &'a mut SimpleSession,
}
impl<'a> Dungeons<'a> {
    // Accepts a mutable reference to SimpleSession
    pub fn new(session: &'a mut SimpleSession) -> Self {
        Dungeons { session }
    }
    pub async fn do_dungeons(&mut self)-> Result<(), Box<dyn std::error::Error>>  {


        loop {
            sleep(Duration::from_secs(2)).await;
            let gs = self.session.send_command(Command::Update).await.unwrap();

            // We might have dungeon keys still waiting to be unlocked, so we
            // should use everything we have
            if let Some(unlockable) = gs.pending_unlocks.first().copied() {
                self.session
                    .send_command(Command::UnlockFeature { unlockable })
                    .await
                    .unwrap();
                continue;
            }

            if let Some(portal) = &gs.dungeons.portal {
                // TODO: I do not have a char, that has finished the portal, so you
                // should maybe check the finished count against the current
                if portal.can_fight {
                    println!("Fighting the player portal");
                    self.session.send_command(Command::FightPortal).await.unwrap();
                    continue;
                }
            }

            if gs.character.inventory.free_slot().is_none() {
                sell_the_worst_item(self.session).await.expect("Error while selling item");
                break;
            }

            let mut best: Option<(Dungeon, &'static Monster)> = None;
            // TODO: ShadowDungeons
            for l in LightDungeon::iter() {
                let Some(current) = gs.dungeons.current_enemy(l) else {
                    continue;
                };
                // You should make a better heuristic to find these, but for now we
                // just find the lowest level
                if best.map_or(true, |old| old.1.level > current.level) {
                    best = Some((l.into(), current))
                }
            }

            let Some((target_dungeon, target_monster)) = best else {
                println!("There are no more enemies left to fight");
                break;
            };


            log_to_file("Chose: {target_dungeon:?} as the best dungeon to fight in").await?;

            let Some(next_fight) = gs.dungeons.next_free_fight else {
                log_to_file("We do not have a time for the next fight").await?;
                break;
            };
            let rem = time_remaining(next_fight);

            if rem > Duration::from_secs(60 * 5)
                && gs.character.mushrooms > 1000
                && target_monster.level <= gs.character.level + 20
            {
                // You should add some better logic on when to skip this
                log_to_file("Using mushrooms to fight in the dungeon").await?;
                self.session
                    .send_command(Command::FightDungeon {
                        dungeon: target_dungeon,
                        use_mushroom: true,
                    })
                    .await
                    .unwrap();
            } else {
                break;
            }
        }
        Ok(())
    }

}




