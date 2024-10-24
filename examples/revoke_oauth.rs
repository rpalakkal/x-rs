use x_rs::account::Account;

#[tokio::main]
async fn main() {
    env_logger::init();
    let account = Account::from_file("auth.txt").unwrap();
    let oauth_applications = account.get_all_oauth_applications().await.unwrap();
    log::info!("{:?}", oauth_applications);
    account.revoke_all_oauth_applications().await.unwrap();
    let new_oauth_applications = account.get_all_oauth_applications().await.unwrap();
    log::info!("{:?}", new_oauth_applications);
}
