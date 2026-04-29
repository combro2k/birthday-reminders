use std::sync::Arc;

use axum::{
    extract::{Extension, Path, State},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::domain::user::{Role, User};
use crate::infrastructure::auth::session::get_csrf_token;
use crate::interface::web::server::AppState;
use crate::interface::web::templates::AdminUsersTemplate;

pub async fn users_page(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
) -> impl IntoResponse {
    if !user.is_admin() {
        return Redirect::to("/").into_response();
    }

    let csrf_token = get_csrf_token(&session).await;
    let users = state
        .user_repo
        .find_all()
        .await
        .unwrap_or_default();

    let template = AdminUsersTemplate {
        user,
        users,
        error: None,
        success: None,
        csrf_token,
    };
    Html(template.to_string()).into_response()
}

#[derive(Deserialize)]
pub struct CreateUserForm {
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: String,
}

pub async fn create_user(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
    Form(form): Form<CreateUserForm>,
) -> impl IntoResponse {
    if !user.is_admin() {
        return Redirect::to("/").into_response();
    }

    let role = Role::from_str(&form.role);

    match state
        .user_command_service
        .create_user(&form.username, &form.email, &form.password, role)
        .await
    {
        Ok(_) => Redirect::to("/admin/users").into_response(),
        Err(e) => {
            let csrf_token = get_csrf_token(&session).await;
            let users = state.user_repo.find_all().await.unwrap_or_default();
            Html(
                AdminUsersTemplate {
                    user,
                    users,
                    error: Some(e.to_string()),
                    success: None,
                    csrf_token,
                }
                .to_string(),
            )
            .into_response()
        }
    }
}

pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(target_id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    if !user.is_admin() {
        return Redirect::to("/").into_response();
    }

    // Prevent self-deletion
    if user.id.0 == target_id {
        return Redirect::to("/admin/users").into_response();
    }

    let _ = state
        .user_command_service
        .delete_user(&target_id.into())
        .await;
    Redirect::to("/admin/users").into_response()
}
