use crate::quest::Quest;

pub const SERVICE_NAME: &str = "quest-tracker";

#[tarpc::service]
pub trait QuestTrackerRPC {
    async fn check_out_quest() -> Option<Quest>;
}
