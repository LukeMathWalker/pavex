pub trait WithId {
    fn id(&self) -> u64;
}

impl WithId for crate::User {
    fn id(&self) -> u64 {
        todo!()
    }
}
