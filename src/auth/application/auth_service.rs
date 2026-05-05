use std::sync::Arc;
use tracing::info;

use crate::auth::infrastructure::oidc::{OidcClient, OidcFlowState, OidcUserInfo};
use crate::auth::infrastructure::password;
use crate::infrastructure::error::RepositoryError;
use crate::users::domain::repository::{NewUser, UserRepository};
use crate::users::domain::user::{AuthMethod, Role, User};

pub struct AuthService {
    user_repo: Arc<dyn UserRepository>,
    oidc_client: Option<Arc<OidcClient>>,
    auto_provision: bool,
    default_role: String,
}

impl AuthService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        oidc_client: Option<Arc<OidcClient>>,
        auto_provision: bool,
        default_role: String,
    ) -> Self {
        Self {
            user_repo,
            oidc_client,
            auto_provision,
            default_role,
        }
    }

    /// Authenticate with username and password
    pub async fn login_local(&self, username: &str, password_input: &str) -> anyhow::Result<User> {
        let user = self
            .user_repo
            .find_by_username(username)
            .await
            .map_err(|_| anyhow::anyhow!("Invalid username or password"))?;

        if !user.auth_method.can_login_with_password() {
            anyhow::bail!("This account does not support password login");
        }

        let hash = user
            .password_hash
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Invalid username or password"))?;

        if !password::verify_password(password_input, hash) {
            anyhow::bail!("Invalid username or password");
        }

        Ok(user)
    }

    /// Get OIDC authorization URL and flow state
    pub fn initiate_oidc(&self) -> anyhow::Result<(String, OidcFlowState)> {
        let client = self
            .oidc_client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("OIDC is not configured"))?;
        Ok(client.authorize_url())
    }

    /// Handle OIDC callback: exchange code, find or create user
    pub async fn handle_oidc_callback(
        &self,
        code: &str,
        flow_state: &OidcFlowState,
    ) -> anyhow::Result<User> {
        let client = self
            .oidc_client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("OIDC is not configured"))?;

        let user_info = client.exchange_code(code, flow_state).await?;

        // Try to find existing user by OIDC subject
        match self
            .user_repo
            .find_by_oidc_subject(&user_info.subject)
            .await
        {
            Ok(user) => Ok(user),
            Err(RepositoryError::NotFound) => {
                if !self.auto_provision {
                    anyhow::bail!("Account not provisioned. Please contact an administrator.");
                }
                self.provision_oidc_user(&user_info).await
            }
            Err(e) => Err(anyhow::anyhow!("Database error: {}", e)),
        }
    }

    /// Create a new user from OIDC info
    async fn provision_oidc_user(&self, info: &OidcUserInfo) -> anyhow::Result<User> {
        let username = info
            .preferred_username
            .clone()
            .or_else(|| info.name.clone())
            .or_else(|| info.email.clone())
            .unwrap_or_else(|| info.subject.clone());

        let email = info
            .email
            .clone()
            .unwrap_or_else(|| format!("{}@oidc", info.subject));

        let role = Role::from_str(&self.default_role);

        let new_user = NewUser {
            username,
            email,
            password_hash: None,
            role,
            auth_method: AuthMethod::Oidc,
            oidc_subject: Some(info.subject.clone()),
        };

        let user = self
            .user_repo
            .create(new_user)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create OIDC user: {}", e))?;

        Ok(user)
    }

    /// Create a default admin user if no users exist in the system.
    /// This is intended to be called on application startup.
    pub async fn bootstrap_admin_user(&self) -> anyhow::Result<()> {
        let users = self
            .user_repo
            .find_all()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to check existing users: {}", e))?;

        if !users.is_empty() {
            return Ok(());
        }

        info!("No users found in database. Bootstrapping default admin account.");

        // Generate a random 12-character password using a UUID
        let password = uuid::Uuid::new_v4().to_string().replace("-", "")[..12].to_string();
        let password_hash = password::hash_password(&password)?;

        let new_user = NewUser {
            username: "admin".to_string(),
            email: "admin@example.org".to_string(),
            password_hash: Some(password_hash),
            role: Role::from_str("admin"),
            auth_method: AuthMethod::Local,
            oidc_subject: None,
        };

        self.user_repo
            .create(new_user)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create default admin: {}", e))?;

        info!("**********************************************************");
        info!("DEFAULT ADMIN USER CREATED");
        info!("Username: admin");
        info!("Password: {}", password);
        info!("Email:    admin@example.org");
        info!("**********************************************************");
        info!("Please change this password immediately after logging in.");

        Ok(())
    }
}
