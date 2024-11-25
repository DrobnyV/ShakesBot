use sf_api::SimpleSession;

pub struct Arena<'a> {
    session: &'a mut SimpleSession,
}
impl<'a> Arena<'a> {
    // Accepts a mutable reference to SimpleSession
    pub fn new(session: &'a mut SimpleSession) -> Self {
        Arena { session }
    }
    pub async fn fight_arena(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}