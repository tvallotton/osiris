use std::time::Duration;

#[derive(Clone, Copy)]
pub struct Stats;
impl Stats {
    pub const fn new() -> Stats {
        Stats
    }

    pub fn register_request(&mut self) {
        todo!()
    }

    pub fn completion(&mut self, time: Duration) {
        todo!()
    }

    fn rotate(&mut self) {
        // if self.0.timestamp.elapsed().as_secs() > 1 {
        //     self.0 = self.1;
        //     self.1 = self.2;
        //     self.2 = Record {
        //         requested: 0,
        //         completed: 0,
        //         timestamp: Instant::now(),
        //     };
        // }
    }
}
