use crate::error::ApiError;
use crate::state::AppState;
use axum::http::{header, HeaderMap, HeaderValue, Method};
use meetoops_core::AuthenticatedSession;
use meetoops_security::{constant_time_eq, has_permission, Permission};

pub const SESSION_COOKIE_NAME: &str = "meetoops_session";

pub fn extract_cookie_token(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in cookie_header.split(';') {
        let trimmed = part.trim();
        if let Some((name, val)) = trimmed.split_once('=') {
            if name.trim() == SESSION_COOKIE_NAME && !val.trim().is_empty() {
                return Some(val.trim().to_string());
            }
        }
    }
    None
}

pub fn build_session_cookie(token: &str, max_age_secs: i64, secure: bool) -> HeaderValue {
    let secure_flag = if secure { "; Secure" } else { "" };
    let val = format!(
        "{SESSION_COOKIE_NAME}={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={max_age_secs}{secure_flag}"
    );
    HeaderValue::from_str(&val).expect("valid set-cookie header")
}

pub fn build_clear_session_cookie(secure: bool) -> HeaderValue {
    let secure_flag = if secure { "; Secure" } else { "" };
    let val = format!(
        "{SESSION_COOKIE_NAME}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0{secure_flag}"
    );
    HeaderValue::from_str(&val).expect("valid clear-cookie header")
}

pub fn is_https_request(headers: &HeaderMap, trust_proxy: bool) -> bool {
    if !trust_proxy {
        return false;
    }
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("https"))
        .unwrap_or(false)
}

pub fn client_ip(headers: &HeaderMap, trust_proxy: bool) -> String {
    if trust_proxy {
        if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
            if let Some(first) = xff.split(',').next() {
                let ip = first.trim();
                if !ip.is_empty() {
                    return ip.to_string();
                }
            }
        }
    }
    "local".to_string()
}

/// Validates either `Authorization: Bearer mto_...` API token OR session cookie + `X-CSRF-Token`.
pub async fn require_auth(
    state: &AppState,
    headers: &HeaderMap,
    method: &Method,
) -> Result<AuthenticatedSession, ApiError> {
    // 1. Check Bearer API Token (exempt from CSRF)
    if let Some(auth_hdr) = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        if let Some(bearer) = auth_hdr.strip_prefix("Bearer ").map(str::trim) {
            if bearer.starts_with("mto_") {
                let user = state
                    .store
                    .resolve_api_token(bearer)
                    .await?
                    .ok_or_else(|| ApiError::unauthorized("Invalid or expired API token."))?;
                return Ok(AuthenticatedSession {
                    session_id: "api_token".to_string(),
                    csrf_token: String::new(),
                    user,
                });
            }
        }
    }

    // 2. Session Cookie + constant-time CSRF check
    let raw_token = extract_cookie_token(headers)
        .ok_or_else(|| ApiError::unauthorized("Authentication required."))?;

    let session = state
        .store
        .resolve_session(&raw_token)
        .await?
        .ok_or_else(|| ApiError::unauthorized("Session expired or invalid."))?;

    if matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    ) {
        let provided_csrf = headers
            .get("x-csrf-token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !constant_time_eq(provided_csrf, &session.csrf_token) {
            return Err(ApiError::forbidden("Invalid or missing CSRF token."));
        }
    }

    Ok(session)
}

pub async fn require_permission(
    state: &AppState,
    headers: &HeaderMap,
    method: &Method,
    permission: Permission,
) -> Result<AuthenticatedSession, ApiError> {
    let session = require_auth(state, headers, method).await?;
    if !has_permission(session.user.role, permission) {
        return Err(ApiError::forbidden(format!(
            "Insufficient permissions ({}).",
            permission.as_str()
        )));
    }
    Ok(session)
}
