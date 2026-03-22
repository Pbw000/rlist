use super::driver::local::local::LocalStorage;
use super::driver::mcloud::client::McloudStorage;

pub enum AllDriver {
    Mcloud(McloudStorage),
    LocalStorage(LocalStorage),
}
