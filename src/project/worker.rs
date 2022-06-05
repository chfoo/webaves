use super::unit::{WorkUnitSource, WorkUnit};

pub struct Worker<T: WorkUnitSource> {
    concurrency: usize,
    work_unit_source: T,
}

impl<T: WorkUnitSource> Worker<T> {
    pub fn concurrency(&self) -> usize {
        self.concurrency
    }

    pub fn set_concurrency(&mut self, concurrency: usize) {
        self.concurrency = concurrency;
    }

    pub async fn run(&mut self) {
        loop {
            self.work_unit_source.get_work_unit().await;
        }
    }

    async fn process_work_unit(&self, work_unit:WorkUnit) {
        todo!()
    }
}
