use std::path::Path;

use super::identity_manager::EncryptionConfiguration;
use super::IdentityConfiguration;
use crate::lib::error::DfxResult;
use crate::lib::identity::identity_file_locations::IdentityFileLocations;
use crate::lib::identity::keyring_mock;
use crate::lib::identity::pem_safekeeping::PromptMode::{DecryptingToUse, EncryptingToCreate};
use dfx_core::error::encryption::EncryptionError;
use dfx_core::error::encryption::EncryptionError::{DecryptContentFailed, HashPasswordFailed};
use dfx_core::error::identity::IdentityError;
use dfx_core::error::identity::IdentityError::{
    DecryptPemFileFailed, LoadPemFromKeyringFailed, ReadPemFileFailed,
};
use dfx_core::error::io::IoError;

use aes_gcm::aead::{Aead, NewAead};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use anyhow::bail;
use argon2::{password_hash::PasswordHasher, Argon2};
use fn_error_context::context;
use slog::{debug, trace, Logger};

/// Loads an identity's PEM file content.
pub(crate) fn load_pem(
    log: &Logger,
    locations: &IdentityFileLocations,
    identity_name: &str,
    identity_config: &IdentityConfiguration,
) -> Result<(Vec<u8>, bool), IdentityError> {
    if identity_config.hsm.is_some() {
        unreachable!("Cannot load pem content for an HSM identity.")
    } else if identity_config.keyring_identity_suffix.is_some() {
        debug!(
            log,
            "Found keyring identity suffix - PEM file is stored in keyring."
        );
        let pem = keyring_mock::load_pem_from_keyring(identity_name)
            .map_err(|err| LoadPemFromKeyringFailed(Box::new(identity_name.to_string()), err))?;
        Ok((pem, true))
    } else {
        let pem_path = locations.get_identity_pem_path(identity_name, identity_config);
        load_pem_from_file(&pem_path, Some(identity_config))
    }
}

#[context("Failed to save PEM file for identity '{name}'.")]
pub(crate) fn save_pem(
    log: &Logger,
    locations: &IdentityFileLocations,
    name: &str,
    identity_config: &IdentityConfiguration,
    pem_content: &[u8],
) -> DfxResult<()> {
    trace!(
        log,
        "Saving pem with input identity name '{name}' and identity config {:?}",
        identity_config
    );
    if identity_config.hsm.is_some() {
        bail!("Cannot save PEM content for an HSM.")
    } else if let Some(keyring_identity) = &identity_config.keyring_identity_suffix {
        debug!(log, "Saving keyring identity.");
        keyring_mock::write_pem_to_keyring(keyring_identity, pem_content)?;
        Ok(())
    } else {
        let path = locations.get_identity_pem_path(name, identity_config);
        write_pem_to_file(&path, Some(identity_config), pem_content)?;
        Ok(())
    }
}

/// Loads a pem file, no matter if it is a plaintext pem file or if it is encrypted with a password.
/// Transparently handles all complexities regarding pem file encryption, including prompting the user for the password.
/// Returns the pem and whether the original was encrypted.
///
/// Try to only load the pem file once, as the user may be prompted for the password every single time you call this function.
pub fn load_pem_from_file(
    path: &Path,
    config: Option<&IdentityConfiguration>,
) -> Result<(Vec<u8>, bool), IdentityError> {
    let content = dfx_core::fs::read(path).map_err(ReadPemFileFailed)?;

    let (content, was_encrypted) = maybe_decrypt_pem(content.as_slice(), config)
        .map_err(|err| DecryptPemFileFailed(path.to_path_buf(), err))?;
    Ok((content, was_encrypted))
}

/// Transparently handles all complexities regarding pem file encryption, including prompting the user for the password.
///
/// Automatically creates required directories.
pub fn write_pem_to_file(
    path: &Path,
    config: Option<&IdentityConfiguration>,
    pem_content: &[u8],
) -> Result<(), IdentityError> {
    let pem_content = maybe_encrypt_pem(pem_content, config)
        .map_err(|err| IdentityError::EncryptPemFileFailed(path.to_path_buf(), err))?;

    write_pem_content(path, &pem_content).map_err(IdentityError::WritePemFileFailed)
}

fn write_pem_content(path: &Path, pem_content: &[u8]) -> Result<(), IoError> {
    let containing_folder = dfx_core::fs::parent(path)?;
    dfx_core::fs::create_dir_all(&containing_folder)?;
    dfx_core::fs::write(path, pem_content)?;

    let mut permissions = dfx_core::fs::read_permissions(path)?;

    permissions.set_readonly(true);
    // On *nix, set the read permission to owner-only.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        permissions.set_mode(0o400);
    }

    dfx_core::fs::set_permissions(path, permissions)
}

/// If the IndentityConfiguration suggests that the content of the pem file should be encrypted,
/// then the user is prompted for the password to the pem file.
/// The encrypted pem file content is then returned.
///
/// If the pem file should not be encrypted, then the content is returned as is.
///
/// `maybe_decrypt_pem` does the opposite.
fn maybe_encrypt_pem(
    pem_content: &[u8],
    config: Option<&IdentityConfiguration>,
) -> Result<Vec<u8>, EncryptionError> {
    if let Some(encryption_config) = config.and_then(|c| c.encryption.as_ref()) {
        let password = password_prompt(EncryptingToCreate)?;
        let result = encrypt(pem_content, encryption_config, &password);
        println!("Encryption complete.");
        result
    } else {
        Ok(Vec::from(pem_content))
    }
}

/// If the IndentityConfiguration suggests that the content of the pem file is encrypted,
/// then the user is prompted for the password to the pem file.
/// The decrypted pem file content is then returned.
///
/// If the pem file should not be encrypted, then the content is returned as is.
///
/// Additionally returns whether or not it was necessary to decrypt the file.
///
/// `maybe_encrypt_pem` does the opposite.
fn maybe_decrypt_pem(
    pem_content: &[u8],
    config: Option<&IdentityConfiguration>,
) -> Result<(Vec<u8>, bool), EncryptionError> {
    if let Some(decryption_config) = config.and_then(|c| c.encryption.as_ref()) {
        let password = password_prompt(DecryptingToUse)?;
        let pem = decrypt(pem_content, decryption_config, &password)?;
        // print to stderr so that output redirection works for the identity export command
        eprintln!("Decryption complete.");
        Ok((pem, true))
    } else {
        Ok((Vec::from(pem_content), false))
    }
}

enum PromptMode {
    EncryptingToCreate,
    DecryptingToUse,
}

fn password_prompt(mode: PromptMode) -> Result<String, EncryptionError> {
    let prompt = match mode {
        PromptMode::EncryptingToCreate => "Please enter a passphrase for your identity",
        PromptMode::DecryptingToUse => "Please enter the passphrase for your identity",
    };
    dialoguer::Password::new()
        .with_prompt(prompt)
        .interact()
        .map_err(EncryptionError::ReadUserPasswordFailed)
}

fn get_argon_params() -> argon2::Params {
    argon2::Params::new(64000 /* in kb */, 3, 1, Some(32 /* in bytes */)).unwrap()
}

fn encrypt(
    content: &[u8],
    config: &EncryptionConfiguration,
    password: &str,
) -> Result<Vec<u8>, EncryptionError> {
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        get_argon_params(),
    );
    let hash = argon2
        .hash_password(password.as_bytes(), &config.pw_salt)
        .map_err(EncryptionError::HashPasswordFailed)?;
    let key = Key::clone_from_slice(hash.hash.unwrap().as_ref());
    let cipher = Aes256Gcm::new(&key);
    let nonce = Nonce::from_slice(config.file_nonce.as_slice());

    let encrypted = cipher
        .encrypt(nonce, content)
        .map_err(EncryptionError::EncryptContentFailed)?;

    Ok(encrypted)
}

fn decrypt(
    encrypted_content: &[u8],
    config: &EncryptionConfiguration,
    password: &str,
) -> Result<Vec<u8>, EncryptionError> {
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        get_argon_params(),
    );
    let hash = argon2
        .hash_password(password.as_bytes(), &config.pw_salt)
        .map_err(HashPasswordFailed)?;
    let key = Key::clone_from_slice(hash.hash.unwrap().as_ref());
    let cipher = Aes256Gcm::new(&key);
    let nonce = Nonce::from_slice(config.file_nonce.as_slice());

    cipher
        .decrypt(nonce, encrypted_content.as_ref())
        .map_err(DecryptContentFailed)
}

#[cfg(test)]
mod test {
    use super::*;
    use proptest::prelude::*;
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(90))] // takes ~0.3s per case
        #[test]
        fn decrypt_reverts_encrypt(pass in ".*", content in ".*") {
            let config = EncryptionConfiguration::new().unwrap();
            let encrypted = encrypt(content.as_bytes(), &config, &pass).unwrap();
            let decrypted = decrypt(&encrypted, &config, &pass).unwrap();

            assert_eq!(content.as_bytes(), decrypted.as_slice());
        }
    }
}
