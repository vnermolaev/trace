use std::collections::HashMap;
use std::time::Instant;
use trace::trace;

fn main() {
    env_logger::init();

    let mut tracker = Tracker::new();
    let _ = tracker.record(&[1, 2, 3]);
}

struct Tracker {
    inner: HashMap<Vec<u8>, Instant>,
}

#[trace(disable(new), prefix = "Tracker::")]
impl Tracker {
    fn new() -> Self {
        Tracker {
            inner: HashMap::new(),
        }
    }

    // This function will not work if its internals are captured differently than FnMut.
    fn record(&mut self, key: &[u8]) -> Option<usize> {
        let new = !self.inner.contains_key(key);
        *self
            .inner
            .entry(Vec::from(key))
            .or_insert_with(Instant::now) = Instant::now();
        if new {
            Some(self.inner.len())
        } else {
            None
        }
    }
}
