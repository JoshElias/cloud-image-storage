use std::fmt::Write;

use anyhow::anyhow;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::config::AuthConfig;

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

pub struct NewSession {
    pub token: String,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| anyhow!("hash password: {error}"))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(password: &str, password_hash: &str) -> anyhow::Result<bool> {
    let parsed_hash =
        PasswordHash::new(password_hash).map_err(|error| anyhow!("parse hash: {error}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn new_session(config: &AuthConfig) -> NewSession {
    let token = format!("{}.{}", Uuid::new_v4(), Uuid::new_v4());
    let token_hash = hash_session_token(&token);
    let expires_at = Utc::now() + Duration::hours(config.session_ttl_hours);

    NewSession {
        token,
        token_hash,
        expires_at,
    }
}

pub fn session_cookie(headers: &HeaderMap, config: &AuthConfig) -> Option<String> {
    let header = headers.get(COOKIE)?.to_str().ok()?;

    for cookie in header.split(';') {
        let cookie = cookie.trim();
        let (name, value) = cookie.split_once('=')?;

        if name == config.cookie_name && !value.is_empty() {
            return Some(value.to_string());
        }
    }

    None
}

pub fn hash_session_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    let mut encoded = String::with_capacity(digest.len() * 2);

    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("write to string cannot fail");
    }

    encoded
}

pub fn set_session_cookie(
    headers: &mut HeaderMap,
    config: &AuthConfig,
    token: &str,
) -> anyhow::Result<()> {
    let mut cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        config.cookie_name,
        token,
        config.session_ttl_hours * 60 * 60
    );

    if config.cookie_secure {
        cookie.push_str("; Secure");
    }

    headers.insert(SET_COOKIE, HeaderValue::from_str(&cookie)?);
    Ok(())
}

pub fn clear_session_cookie(headers: &mut HeaderMap, config: &AuthConfig) -> anyhow::Result<()> {
    let mut cookie = format!(
        "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
        config.cookie_name
    );

    if config.cookie_secure {
        cookie.push_str("; Secure");
    }

    headers.insert(SET_COOKIE, HeaderValue::from_str(&cookie)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn hashes_and_verifies_passwords() {
        let hash = hash_password("correct horse battery staple").expect("hash password");

        assert!(verify_password("correct horse battery staple", &hash).expect("verify password"));
        assert!(!verify_password("wrong", &hash).expect("verify password"));
    }

    #[test]
    fn reads_named_session_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_static("other=value; cis_session=abc123; theme=light"),
        );
        let config = AuthConfig::default();

        assert_eq!(session_cookie(&headers, &config).as_deref(), Some("abc123"));
    }
}
