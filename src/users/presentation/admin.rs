use std::sync::Arc;

use axum::{
    Form,
    extract::{Extension, Path, State},
    response::{Html, IntoResponse, Redirect},
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::auth::infrastructure::session::get_csrf_token;
use crate::infrastructure::web::server::AppState;
use crate::users::domain::user::{Role, User};
use crate::users::presentation::templates::{AdminUsersTemplate, UserView};

pub async fn users_page(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
) -> impl IntoResponse {
    if !user.is_admin() {
        return Redirect::to("/").into_response();
    }

    let csrf_token = get_csrf_token(&session).await;
    let users = state.user_repo.find_all().await.unwrap_or_default();

    let user_views = users
        .into_iter()
        .map(|u| UserView::from_user(u, &user.date_format))
        .collect();

    let template = AdminUsersTemplate {
        user,
        users: user_views,
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
            let user_views = users
                .into_iter()
                .map(|u| UserView::from_user(u, &user.date_format))
                .collect();
            Html(
                AdminUsersTemplate {
                    user,
                    users: user_views,
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
