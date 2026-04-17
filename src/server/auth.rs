use std::sync::Arc;

use axum::extract::FromRef;
use axum::{
    extract::{Query, Request, State},
    http::{HeaderMap, StatusCode, Uri},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::{Cookie, Key, PrivateCookieJar, SameSite};
use cookie::time::Duration;
use dioxus::prelude::ServerFnError;
use dioxus_fullstack_core::FullstackContext;
use openidconnect::{core::CoreProviderMetadata, reqwest::Client as OidcHttpClient, IssuerUrl};
use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};

use crate::server::{config, db};

pub const DEFAULT_POST_LOGIN_PATH: &str = "/admin";
const GAMMA_SCOPES: &str = "openid profile";
const SESSION_COOKIE_NAME: &str = "findit_session";

#[derive(Clone)]
pub struct AuthState {
    pub cookie_key: Key,
    pub cookie_name: String,
    pub session_ttl_hours: i64,
    pub oidc_client_id: String,
    pub oidc_client_secret: String,
    pub oidc_redirect_url: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub oidc_http_client: reqwest::Client,
}

#[derive(Clone, Debug)]
pub struct AuthSession {
    pub display_name: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RequestAuth(pub Option<AuthSession>);

#[derive(Deserialize)]
pub struct LoginQuery {
    pub next: Option<String>,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

#[derive(Clone)]
pub struct CookieKey(pub Key);

impl FromRef<Arc<AuthState>> for CookieKey {
    fn from_ref(state: &Arc<AuthState>) -> Self {
        Self(state.cookie_key.clone())
    }
}

impl From<CookieKey> for Key {
    fn from(value: CookieKey) -> Self {
        value.0
    }
}

pub async fn build_auth_state(
) -> Result<Arc<AuthState>, Box<dyn std::error::Error + Send + Sync>> {
    let cfg = config::get();
    let discovery_http_client = OidcHttpClient::builder().redirect(Policy::none()).build()?;
    let oidc_http_client = reqwest::Client::builder()
        .redirect(Policy::limited(10))
        .build()?;

    let provider_metadata = CoreProviderMetadata::discover_async(
        IssuerUrl::new(cfg.oidc_issuer_url.clone())?,
        &discovery_http_client,
    )
    .await?;

    let authorization_endpoint = provider_metadata.authorization_endpoint().clone();
    let token_endpoint = provider_metadata
        .token_endpoint()
        .cloned()
        .ok_or("OIDC provider metadata did not include a token endpoint")?;
    let userinfo_endpoint = provider_metadata
        .userinfo_endpoint()
        .cloned()
        .ok_or("OIDC provider metadata did not include a userinfo endpoint")?;

    Ok(Arc::new(AuthState {
        cookie_key: Key::from(
            derived_cookie_key(cfg.session_cookie_secret.as_bytes()).as_slice(),
        ),
        cookie_name: SESSION_COOKIE_NAME.to_string(),
        session_ttl_hours: cfg.session_ttl_hours,
        oidc_client_id: cfg.oidc_client_id.clone(),
        oidc_client_secret: cfg.oidc_client_secret.clone(),
        oidc_redirect_url: cfg.oidc_redirect_url.clone(),
        authorization_endpoint: authorization_endpoint.to_string(),
        token_endpoint: token_endpoint.to_string(),
        userinfo_endpoint: userinfo_endpoint.to_string(),
        oidc_http_client,
    }))
}

pub async fn auth_middleware(
    State(auth_state): State<Arc<AuthState>>,
    mut request: Request,
    next: Next,
) -> Response {
    let session = load_session_from_headers(&request.headers().clone(), &auth_state).await;
    request.extensions_mut().insert(RequestAuth(session));
    next.run(request).await
}

pub async fn login_handler(
    State(auth_state): State<Arc<AuthState>>,
    Query(query): Query<LoginQuery>,
) -> Result<impl IntoResponse, Response> {
    let next = sanitize_next(query.next.as_deref());
    let state = random_token();
    let auth_url = build_authorization_url(&auth_state, &state).map_err(internal_response)?;

    db::create_oidc_login_attempt(db::pool(), &state, "", "", &next)
        .await
        .map_err(internal_response)?;

    Ok(Redirect::to(&auth_url))
}

pub async fn callback_handler(
    State(auth_state): State<Arc<AuthState>>,
    jar: PrivateCookieJar<CookieKey>,
    Query(query): Query<CallbackQuery>,
) -> Result<Response, Response> {
    let attempt = db::consume_oidc_login_attempt(db::pool(), &query.state)
        .await
        .map_err(internal_response)?
        .ok_or_else(|| unauthorized_response("Login attempt not found or expired"))?;

    let token_response = exchange_code(&auth_state, query.code)
        .await
        .map_err(internal_response)?;
    let userinfo = fetch_userinfo(&auth_state, &token_response.access_token)
        .await
        .map_err(internal_response)?;

    let subject = userinfo.sub;
    let issuer = config::get().oidc_issuer_url.clone();
    let display_name = userinfo
        .name
        .or(userinfo.preferred_username)
        .or(userinfo.nickname)
        .or(userinfo.email);

    let session_token = random_token();

    db::create_auth_session(
        db::pool(),
        &session_token,
        &subject,
        &issuer,
        display_name.as_deref(),
        auth_state.session_ttl_hours,
    )
    .await
    .map_err(internal_response)?;

    let cookie = build_session_cookie(&auth_state, session_token);
    Ok((jar.add(cookie), Redirect::to(&attempt.next_path)).into_response())
}

pub async fn logout_handler(
    State(auth_state): State<Arc<AuthState>>,
    jar: PrivateCookieJar<CookieKey>,
) -> Result<Response, Response> {
    if let Some(cookie) = jar.get(&auth_state.cookie_name) {
        let token = cookie.value().to_string();
        let _ = db::delete_auth_session(db::pool(), &token).await;
    }

    let mut removal = Cookie::new(auth_state.cookie_name.clone(), String::new());
    removal.set_path("/");
    Ok((jar.remove(removal), Redirect::to("/")).into_response())
}

pub async fn require_authenticated_request() -> Result<AuthSession, ServerFnError> {
    require_optional_session()
        .await?
        .ok_or_else(|| ServerFnError::new("Authentication required"))
}

pub async fn require_optional_session() -> Result<Option<AuthSession>, ServerFnError> {
    let ctx = FullstackContext::current()
        .ok_or_else(|| ServerFnError::new("Missing request context"))?;
    if let Some(auth) = ctx.extension::<RequestAuth>() {
        Ok(auth.0)
    } else {
        let headers = ctx.parts_mut().headers.clone();
        let state = build_auth_state()
            .await
            .map_err(|err| ServerFnError::new(format!("Auth setup error: {err}")))?;
        Ok(load_session_from_headers(&headers, &state).await)
    }
}

fn build_session_cookie(auth_state: &AuthState, value: String) -> Cookie<'static> {
    let mut cookie = Cookie::new(auth_state.cookie_name.clone(), value);
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_secure(is_secure_cookie());
    cookie.set_max_age(Duration::hours(auth_state.session_ttl_hours));
    cookie
}

async fn load_session_from_headers(
    headers: &HeaderMap,
    auth_state: &AuthState,
) -> Option<AuthSession> {
    let jar = PrivateCookieJar::from_headers(headers, auth_state.cookie_key.clone());
    let token = jar.get(&auth_state.cookie_name)?.value().to_string();
    db::get_auth_session_by_token(db::pool(), &token)
        .await
        .ok()
        .flatten()
}

fn sanitize_next(next: Option<&str>) -> String {
    let next = next.unwrap_or(DEFAULT_POST_LOGIN_PATH);
    if next.starts_with('/') && !next.starts_with("//") {
        next.to_string()
    } else {
        DEFAULT_POST_LOGIN_PATH.to_string()
    }
}

fn random_token() -> String {
    use rand::{distributions::Alphanumeric, thread_rng, Rng};
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

fn derived_cookie_key(secret: &[u8]) -> [u8; 64] {
    use sha2::{Digest, Sha512};

    let digest = Sha512::digest(secret);
    let mut key = [0_u8; 64];
    key.copy_from_slice(&digest);
    key
}

#[derive(Serialize)]
struct GammaTokenRequest {
    client_id: String,
    client_secret: String,
    code: String,
    redirect_uri: String,
    grant_type: String,
}

#[derive(Deserialize)]
struct GammaTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct GammaUserInfo {
    sub: String,
    email: Option<String>,
    nickname: Option<String>,
    preferred_username: Option<String>,
    name: Option<String>,
}

fn build_authorization_url(
    auth_state: &AuthState,
    state: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut url = reqwest::Url::parse(&auth_state.authorization_endpoint)?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &auth_state.oidc_client_id)
        .append_pair("scope", GAMMA_SCOPES)
        .append_pair("redirect_uri", &auth_state.oidc_redirect_url)
        .append_pair("state", state);
    Ok(url.to_string())
}

async fn exchange_code(
    auth_state: &AuthState,
    code: String,
) -> Result<GammaTokenResponse, String> {
    let request_body = GammaTokenRequest {
        client_id: auth_state.oidc_client_id.clone(),
        client_secret: auth_state.oidc_client_secret.clone(),
        code,
        redirect_uri: auth_state.oidc_redirect_url.clone(),
        grant_type: "authorization_code".to_string(),
    };

    let response = auth_state
        .oidc_http_client
        .post(&auth_state.token_endpoint)
        .form(&request_body)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|err| format!("Failed to reach Gamma token endpoint: {err}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| format!("Failed to read Gamma token response: {err}"))?;

    if !status.is_success() {
        return Err(format!(
            "Gamma token endpoint returned {status}. Body: {body}"
        ));
    }

    serde_json::from_str(&body).map_err(|_| {
        "Gamma returned an HTML login page from the token endpoint instead of JSON. This usually means the client is not configured for standard authorization-code token exchange, or the registered redirect URI/client settings in Gamma do not match this app."
            .to_string()
    })
}

async fn fetch_userinfo(
    auth_state: &AuthState,
    access_token: &str,
) -> Result<GammaUserInfo, String> {
    let response = auth_state
        .oidc_http_client
        .get(&auth_state.userinfo_endpoint)
        .bearer_auth(access_token)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|err| format!("Failed to reach Gamma userinfo endpoint: {err}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| format!("Failed to read Gamma userinfo response: {err}"))?;

    if !status.is_success() {
        return Err(format!(
            "Gamma userinfo endpoint returned {status}. Body: {body}"
        ));
    }

    serde_json::from_str(&body)
        .map_err(|err| format!("Failed to parse Gamma userinfo response: {err}"))
}

fn is_secure_cookie() -> bool {
    match config::get().oidc_redirect_url.parse::<Uri>() {
        Ok(uri) => uri.scheme_str() == Some("https"),
        Err(_) => false,
    }
}

fn unauthorized_response(message: impl Into<String>) -> Response {
    (StatusCode::UNAUTHORIZED, message.into()).into_response()
}

fn internal_response(error: impl std::fmt::Display) -> Response {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
}

impl<S> axum::extract::FromRequestParts<S> for RequestAuth
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<RequestAuth>()
            .cloned()
            .unwrap_or(RequestAuth(None)))
    }
}
