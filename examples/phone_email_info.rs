use x_rs::account::Account;

#[tokio::main]
async fn main() {
    env_logger::init();
    let account = Account::from_file("auth.txt").unwrap();
    let phone_email_info = account.get_email_phone_info().await.unwrap();
    log::info!("{:?}", phone_email_info);
}
