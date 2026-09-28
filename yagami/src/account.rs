// Who may take PLATFORM actions: creating and ending games, saving keys. In-game power never comes
// from an account -- only from a game's own keys (see auth.rs).
//
// Credentials never touch the DB in the clear: passwords are stored as argon2id PHC strings, and a
// session token is handed to the client once while only its sha256 is stored.

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub type AccountId = u64; // DB BIGSERIAL id, like GameId.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Role {
    User,
    // manages any game at the platform level and has no per-account game quota.
    Admin,
}

impl Role {
    // the accounts.role column; the migration's CHECK holds it to these two.
    pub fn from_db(role: &str) -> Option<Self> {
        match role {
            "user" => Some(Self::User),
            "admin" => Some(Self::Admin),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Account {
    pub id: AccountId,
    pub username: String,
    pub role: Role,
    pub verified: bool,
}

// argon2 is deliberately slow (tens of ms of CPU), so both directions run off the async workers.
pub async fn hash_password(password: String) -> String {
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes())
            .expect("OS CSPRNG unavailable")
            .to_string()
    })
    .await
    .expect("password hashing panicked")
}

pub async fn verify_password(password: String, hash: String) -> bool {
    tokio::task::spawn_blocking(move || {
        // a hash that doesn't parse can only come from a bad manual edit; it matches nothing.
        let Ok(parsed) = PasswordHash::new(&hash) else {
            eprintln!("unparseable password hash in the accounts table");
            return false;
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
    .await
    .expect("password verification panicked")
}

pub fn session_token_hash(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}
