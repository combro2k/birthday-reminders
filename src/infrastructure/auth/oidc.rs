use openidconnect::{
    core::{CoreAuthenticationFlow, CoreProviderMetadata},
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope,
    TokenResponse,
};
use openidconnect::reqwest;

use crate::infrastructure::config::OidcConfig;

pub struct OidcClient {
    metadata: CoreProviderMetadata,
    client_id: ClientId,
    client_secret: ClientSecret,
    redirect_url: RedirectUrl,
    http_client: reqwest::Client,
    scopes: Vec<String>,
}

/// Data stored in session during OIDC flow
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OidcFlowState {
    pub csrf_token: String,
    pub nonce: String,
    pub pkce_verifier: String,
}

/// User info extracted from OIDC tokens
#[derive(Debug)]
pub struct OidcUserInfo {
    pub subject: String,
    pub email: Option<String>,
    pub preferred_username: Option<String>,
    pub name: Option<String>,
}

impl OidcClient {
    pub async fn new(config: &OidcConfig, base_url: &str) -> anyhow::Result<Self> {
        let http_client = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("HTTP client should build");

        let issuer_url = IssuerUrl::new(config.issuer_url.clone())?;
        let metadata =
            CoreProviderMetadata::discover_async(issuer_url, &http_client).await?;

        let redirect_url = RedirectUrl::new(format!(
            "{}/auth/oidc/callback",
            base_url
        ))?;

        Ok(Self {
            metadata,
            client_id: ClientId::new(config.client_id.clone()),
            client_secret: ClientSecret::new(config.client_secret.clone()),
            redirect_url,
            http_client,
            scopes: config.scopes.clone(),
        })
    }

    /// Generate the authorization URL and flow state to store in session
    pub fn authorize_url(&self) -> (String, OidcFlowState) {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let client = openidconnect::core::CoreClient::from_provider_metadata(
            self.metadata.clone(),
            self.client_id.clone(),
            Some(self.client_secret.clone()),
        )
        .set_redirect_uri(self.redirect_url.clone());

        let mut auth_request = client.authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        );

        for scope in &self.scopes {
            auth_request = auth_request.add_scope(Scope::new(scope.clone()));
        }

        let (auth_url, csrf_token, nonce) = auth_request
            .set_pkce_challenge(pkce_challenge)
            .url();

        let flow_state = OidcFlowState {
            csrf_token: csrf_token.secret().clone(),
            nonce: nonce.secret().clone(),
            pkce_verifier: pkce_verifier.secret().clone(),
        };

        (auth_url.to_string(), flow_state)
    }

    /// Exchange the authorization code for user info
    pub async fn exchange_code(
        &self,
        code: &str,
        flow_state: &OidcFlowState,
    ) -> anyhow::Result<OidcUserInfo> {
        let pkce_verifier = PkceCodeVerifier::new(flow_state.pkce_verifier.clone());

        let client = openidconnect::core::CoreClient::from_provider_metadata(
            self.metadata.clone(),
            self.client_id.clone(),
            Some(self.client_secret.clone()),
        )
        .set_redirect_uri(self.redirect_url.clone());

        let token_response = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .map_err(|e| anyhow::anyhow!("Configuration error: {}", e))?
            .set_pkce_verifier(pkce_verifier)
            .request_async(&self.http_client)
            .await
            .map_err(|e| anyhow::anyhow!("Token exchange failed: {}", e))?;

        // Extract claims from id_token
        let id_token = token_response
            .id_token()
            .ok_or_else(|| anyhow::anyhow!("No id_token in response"))?;

        let nonce = openidconnect::Nonce::new(flow_state.nonce.clone());
        let id_token_verifier = client.id_token_verifier();
        let claims = id_token
            .claims(&id_token_verifier, &nonce)
            .map_err(|e| anyhow::anyhow!("Failed to verify id_token: {}", e))?;

        let subject = claims.subject().to_string();
        let email = claims.email().map(|e| e.as_str().to_string());
        let preferred_username = claims
            .preferred_username()
            .map(|u| u.as_str().to_string());
        let name = claims
            .name()
            .and_then(|n| n.get(None))
            .map(|n| n.as_str().to_string());

        Ok(OidcUserInfo {
            subject,
            email,
            preferred_username,
            name,
        })
    }
}
