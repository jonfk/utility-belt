use notify_rust::Notification;
use std::{env, thread, time};

fn main() {
    let args: Vec<String> = env::args().collect();

    let url = &args[1]; //.expect("failed no arguments");
    dbg!(url);
    let mut ready: bool = is_ready(url);

    while !ready {
        thread::sleep(time::Duration::from_millis(1000));
        ready = is_ready(url);
    }

    Notification::new()
        .summary("What you are waiting for is ready")
        .body(&format!("{} is now ready", url))
        .show()
        .expect("failed notification");
}

fn is_ready(url: &str) -> bool {
    let response = reqwest::get(url).expect("get failed");
    let status = response.status();
    status.is_success()
}
