use url::Url;
use uuid::Uuid;
use async_trait::async_trait;

pub struct WorkUnit {
    pub id: i64,
    pub uuid: Uuid,
    pub url: Url,
}

#[async_trait]
pub trait WorkUnitSource {
    async fn get_work_unit(&mut self) -> Option<WorkUnit>;

    async fn resolve_work_unit(&mut self, work_unit:&WorkUnit);
}
