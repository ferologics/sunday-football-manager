use crate::AppState;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect},
    Form,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use std::sync::Arc;

const AUTH_COOKIE_NAME: &str = "sfm_auth";

/// Check if the request is authenticated
pub fn is_authenticated(jar: &CookieJar, state: &AppState) -> bool {
    // If no password is set, everyone is authenticated
    let Some(ref password) = state.auth_password else {
        return true;
    };

    // Check for valid auth cookie
    jar.get(AUTH_COOKIE_NAME)
        .map(|cookie| cookie.value() == password)
        .unwrap_or(false)
}

/// Login form data
#[derive(serde::Deserialize)]
pub struct LoginForm {
    password: String,
}

/// Handle login POST
pub async fn login(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    let Some(ref password) = state.auth_password else {
        // No password configured, redirect to home
        return (jar, Redirect::to("/")).into_response();
    };

    if form.password == *password {
        // Set auth cookie
        let cookie = Cookie::build((AUTH_COOKIE_NAME, password.clone()))
            .path("/")
            .http_only(true)
            .secure(state.secure_cookies)
            .build();
        (jar.add(cookie), Redirect::to("/")).into_response()
    } else {
        // Wrong password - redirect back with error indicator
        (jar, Redirect::to("/?auth_error=1")).into_response()
    }
}

/// Handle logout POST
pub async fn logout(jar: CookieJar) -> impl IntoResponse {
    let cookie = Cookie::build(AUTH_COOKIE_NAME)
        .path("/")
        .build();
    (jar.remove(cookie), Redirect::to("/"))
}

/// Return 401 Unauthorized response
pub fn unauthorized() -> impl IntoResponse {
    (
        axum::http::StatusCode::UNAUTHORIZED,
        Html("<p class=\"error\">Unauthorized. Please log in.</p>".to_string()),
    )
}
