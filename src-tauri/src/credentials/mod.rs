use keyring::Entry;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const KEYRING_ACCOUNT: &str = "registry";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryCredential {
    pub username: String,
    pub secret: String,
}

pub trait CredentialStore {
    fn save(
        &self,
        profile_id: &str,
        credential: &RegistryCredential,
    ) -> Result<(), CredentialError>;
    fn load(&self, profile_id: &str) -> Result<Option<RegistryCredential>, CredentialError>;
    fn delete(&self, profile_id: &str) -> Result<(), CredentialError>;
}

#[derive(Debug, Default)]
pub struct SystemKeyring;

impl CredentialStore for SystemKeyring {
    fn save(
        &self,
        profile_id: &str,
        credential: &RegistryCredential,
    ) -> Result<(), CredentialError> {
        let entry = Entry::new(&service_name(profile_id), KEYRING_ACCOUNT)?;
        entry.set_password(&serde_json::to_string(credential)?)?;
        Ok(())
    }

    fn load(&self, profile_id: &str) -> Result<Option<RegistryCredential>, CredentialError> {
        let entry = Entry::new(&service_name(profile_id), KEYRING_ACCOUNT)?;
        match entry.get_password() {
            Ok(value) => Ok(Some(serde_json::from_str(&value)?)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn delete(&self, profile_id: &str) -> Result<(), CredentialError> {
        let entry = Entry::new(&service_name(profile_id), KEYRING_ACCOUNT)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}

#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("credential store error: {0}")]
    Store(#[from] keyring::Error),
    #[error("credential payload parse failed: {0}")]
    Parse(#[from] serde_json::Error),
}

fn service_name(profile_id: &str) -> String {
    format!("com.yuhaoxin.registry-manager.profile.{profile_id}")
}
