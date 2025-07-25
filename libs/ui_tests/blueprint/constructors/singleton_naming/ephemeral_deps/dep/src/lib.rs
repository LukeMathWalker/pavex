#[derive(Clone)]
pub struct CrossCrateConflict;

#[pavex::methods]
impl CrossCrateConflict {
    #[singleton]
    pub fn new() -> Self {
        todo!()
    }
}
