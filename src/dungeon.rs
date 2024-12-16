use std::borrow::Borrow;
use std::time::Duration;
use enum_map::{EnumArray, EnumMap};
use sf_api::command::Command;
use sf_api::gamestate::dungeons::{Dungeon, DungeonProgress};
use sf_api::SimpleSession;
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
                    log_to_file("Fighting the player portal").await?;
                    self.session.send_command(Command::FightPortal).await.unwrap();
                    continue;
                }
            }

            if gs.character.inventory.free_slot().is_none() {
                sell_the_worst_item(self.session).await.expect("Error while selling item");
                break;
            }

            // You should make a better heuristic to find these, but for now we just
            // find the lowest level
            let best_light_dungeon = find_lowest_lvl_dungeon(&gs.dungeons.light);
            let best_shadow_dungeon = find_lowest_lvl_dungeon(&gs.dungeons.shadow);

            let (target_dungeon, target_level) =
                match (best_light_dungeon, best_shadow_dungeon) {
                    (Some(x), Some(y)) => {
                        if x.1 < y.1 {
                            x
                        } else {
                            y
                        }
                    }
                    (Some(x), _) => x,
                    (_, Some(x)) => x,
                    (None, None) => {
                        log_to_file("There are no dungeons to fight in!").await?;
                        break;
                    }
                };


            log_to_file("Chose: {target_dungeon:?} as the best dungeon to fight in").await?;

            let Some(next_fight) = gs.dungeons.next_free_fight else {
                log_to_file("We do not have a time for the next fight").await?;
                break;
            };
            let rem = time_remaining(next_fight);

            if rem > Duration::from_secs(60 * 5)
                && gs.character.mushrooms > 1000
                && target_level <= gs.character.level + 20
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
fn find_lowest_lvl_dungeon<T: EnumArray<DungeonProgress> + Into<Dungeon>>(
    dungeons: &EnumMap<T, DungeonProgress>,
) -> Option<(Dungeon, u16)> {
    dungeons
        .iter()
        .filter_map(|a| {
            if let DungeonProgress::Open { level, .. } = a.1 {
                Some((a.0.into(), *level))
            } else {
                None
            }
        })
        .min_by_key(|a| {
            a.1
        })
}



