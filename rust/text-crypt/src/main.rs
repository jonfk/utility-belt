use text_crypt::cli;

use dotenv::dotenv;

fn main() {
    dotenv().ok();
    cli::run();
}
