use dotenv::dotenv;
use rand::{distributions::Alphanumeric, Rng};
use x_rs::account::Account;

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv().ok();

    let old_password = std::env::var("X_PASSWORD").unwrap();
    let new_password: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect();

    let account = Account::from_file("auth.txt").unwrap();
    account
        .change_password(&old_password, &new_password)
        .await
        .unwrap();
    log::info!("Password changed to: {}", new_password);
}
