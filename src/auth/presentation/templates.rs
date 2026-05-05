use askama::Template;

use crate::infrastructure::web::templates::AppVersion;

#[derive(Template)]
#[template(path = "auth/login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
    pub oidc_enabled: bool,
    pub oidc_provider_name: String,
    pub registration_enabled: bool,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "auth/register.html")]
pub struct RegisterTemplate {
    pub error: Option<String>,
    pub csrf_token: String,
}

impl AppVersion for LoginTemplate {}
impl AppVersion for RegisterTemplate {}
