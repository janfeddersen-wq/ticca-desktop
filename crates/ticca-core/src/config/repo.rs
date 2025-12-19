//! Repository trait for configuration storage

use anyhow::Result;

use crate::config::database::ConfigDatabase;
use crate::config::models::{OAuthAccount, Setting};

pub trait ConfigRepo {
    fn get_setting(&self, key: &str) -> Result<Option<Setting>>;
    fn set_setting(&self, key: &str, value: &str) -> Result<()>;
    fn list_oauth_accounts(&self, provider: Option<&str>) -> Result<Vec<OAuthAccount>>;
    fn upsert_oauth_account(&self, account: &OAuthAccount) -> Result<()>;
    fn delete_oauth_account(&self, id: &str) -> Result<bool>;
}

impl ConfigRepo for ConfigDatabase {
    fn get_setting(&self, key: &str) -> Result<Option<Setting>> {
        ConfigDatabase::get_setting(self, key)
    }

    fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        ConfigDatabase::set_setting(self, key, value)
    }

    fn list_oauth_accounts(&self, provider: Option<&str>) -> Result<Vec<OAuthAccount>> {
        ConfigDatabase::list_oauth_accounts(self, provider)
    }

    fn upsert_oauth_account(&self, account: &OAuthAccount) -> Result<()> {
        ConfigDatabase::upsert_oauth_account(self, account)
    }

    fn delete_oauth_account(&self, id: &str) -> Result<bool> {
        ConfigDatabase::delete_oauth_account(self, id)
    }
}
