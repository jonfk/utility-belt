use std::time::Duration;

pub mod scheduler;

const MAX_RETRIES: usize = 20;
const MAX_DELAY_SECONDS: u64 = 600;
const DELAY_SECONDS: u64 = 2;

fn delay(tries: u32) -> Duration {
    let delay = DELAY_SECONDS.pow(tries);
    if delay > MAX_DELAY_SECONDS {
        Duration::from_secs(MAX_DELAY_SECONDS)
    } else {
        Duration::from_secs(delay)
    }
}
