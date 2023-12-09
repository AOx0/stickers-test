use auth::Session;
use axum_extra::extract::{PrivateCookieJar, cookie::Cookie};
use axum_server::tls_rustls::RustlsConfig;
use maud::{html, Markup, DOCTYPE, PreEscaped};
use axum::{Router, routing::get, response::{IntoResponse, Redirect, Response}, extract::{State, Request, Host}, Form, middleware::{self, Next}, http::{StatusCode, Uri}, BoxError};
use state::AppState;
use strum::{EnumIter, IntoEnumIterator};
use surrealdb::opt::auth::Scope;
use axum::handler::HandlerWithoutStateExt;
use tower_http::services::ServeDir;

pub mod pool;
pub mod auth;
pub mod state;
pub mod error;

#[derive(Clone, Copy)]
struct Ports {
    http: u16,
    https: u16,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let surreal = std::env::var("SURREAL").expect("SURREAL must be set");
    let s_size = std::env::var("POOL_SIZE").expect("POOL_SIZE must be set");

    let state = state::AppState::new(pool::SPool::new(surreal.as_str(), s_size.parse::<usize>().unwrap()));

    let ports = Ports {
        http: 80,
        https: 443,
    };

    let config = RustlsConfig::from_pem_file(
        "/Users/alejandro/certificate.pem",
        "/Users/alejandro/private-key.pem",
    )
    .await
    .unwrap();

    tokio::spawn(redirect_http_to_https(ports));

    let auth : Router<AppState> = Router::new()
        .route("/signin", get(signin).post(perform_signin))
        .route("/signup", get(signin))
        .route_layer(middleware::from_fn_with_state(state.clone(), middleware_redirect_already_logged_in));

    let admin : Router<AppState> = Router::new()
        .route("/admin", get(admin))
        .route_layer(middleware::from_fn_with_state(state.clone(), middleware_assert_is_admin));

    let app = Router::new()
        .route("/", get(root))
        .route("/other", get(other))
        .route("/signout", get(perform_signout))
        .merge(admin)
        .merge(auth)
        .fallback_service(ServeDir::new("./static/"))
        .layer(tower_http::compression::CompressionLayer::new())
        .with_state(state);
        
    axum_server::bind_rustls(format!("[::]:{}", ports.https).parse().unwrap(), config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[allow(dead_code)]
async fn redirect_http_to_https(ports: Ports) {
    fn make_https(host: String, uri: Uri, ports: Ports) -> Result<Uri, BoxError> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse().unwrap());
        }

        let https_host = host.replace(&ports.http.to_string(), &ports.https.to_string());
        parts.authority = Some(https_host.parse()?);

        Ok(Uri::from_parts(parts)?)
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(host, uri, ports) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                println!("Redirect error: {:?}", error);
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let listener = tokio::net::TcpListener::bind(format!("[::]:{}", ports.http)).await.unwrap();
    axum::serve(listener, redirect.into_make_service())
        .await
        .unwrap();
}

async fn middleware_redirect_already_logged_in(_: State<AppState>, session: Result<Session, error::Error>, req: Request, next: Next) -> Response {
    if session.is_ok() {
        return Redirect::to("/").into_response();
    }
    
    next.run(req).await
}

async fn middleware_assert_is_admin(_: State<AppState>, session: Result<Session, error::Error>, req: Request, next: Next) -> Response {
    if let Ok(session) = session {
        if session.is_admin() {
            return next.run(req).await;
        }
    }
    
    Redirect::to("/").into_response()
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct UserInfo {
    username: String,
    password: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct User {
    is_admin: Option<bool>,
}

async fn perform_signout(State(state): State<AppState>, jar: PrivateCookieJar) -> impl IntoResponse {
    (
        jar.remove(Cookie::named("token")),
        Redirect::to("/")
    )
}

async fn perform_signin(State(state): State<AppState>, jar: PrivateCookieJar, Form(info): Form<UserInfo>) -> impl IntoResponse {
    let db = state.surreal.get().await.unwrap();
    
    let sign_res = db.signin(Scope {
        namespace: "demo",
        database: "demo",
        scope: "account",
        params: info
    }).await;

    match sign_res {
        Ok(token) => {
            (
                jar.add(Cookie::new("token", token.as_insecure_token().to_string())),
                Redirect::to("/"),
            ).into_response()
        },
        Err(e) => {
            println!("Auth error: {:?}", e);
            (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
        }
    }
}

async fn signin(session: Option<Session>) -> Markup {
    println!("Session: {:?}", session);
    Template(Section::Home, Auth::from(session.as_ref()), html!{
        div.flex.flex-col.justify-center {
            div."flex flex-col items-center space-y-4" {
                h1."text-4xl".font-bold {
                    "Sign in"
                }
                form."flex flex-col space-y-4" method="POST" action="/signin" {
                    input."rounded-md border border-zinc-100/95 dark:border-zinc-800/95 p-2" name="username" type="text" placeholder="Username" {}
                    input."rounded-md border border-zinc-100/95 dark:border-zinc-800/95 p-2" name="password" type="password" placeholder="Password" {}
                    button."rounded-md border border-zinc-100/95 dark:border-zinc-800/95 p-2" type="submit" {
                        "Sign in"
                    }
                }
            }
        }
    })  
}

async fn admin(session: Session) -> Markup {

    Template(Section::Admin, Auth::Admin(&session), html!{
        div."p-4".flex.flex-col {
            h1."text-4xl".font-bold { "Hola, " (session.first_name()) "!" }
        }
    })  
}

async fn root(session: Option<Session>) -> Markup {
    Template(Section::Home, Auth::from(session.as_ref()), html!{
        h1."text-4xl".font-bold ."h-[1000px]" {
            "Hello, world!"
        }
    })  
}

async fn other(session: Option<Session>) -> Markup {
    Template(Section::Other, Auth::from(session.as_ref()), html!{
        h1."text-4xl".font-bold ."h-[1000px]" {
            "Other!"
        }
    })  
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
enum Section {
    Admin,
    Home,
    Other,
}

impl Section {
    fn map_path(&self) -> &'static str {
        match self {
            Self::Admin => "/admin",
            Self::Home => "/",
            Self::Other => "/other",
        }
    }
}

impl maud::Render for Section {
    fn render(&self) -> Markup {
        html! {
            (format!("{:?}", self))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Auth<'a> {
    User(&'a Session),
    Admin(&'a Session),
    Guest,
}


impl<'a> From<Option<&'a Session>> for Auth<'a> {
    fn from(s: Option<&'a Session>) -> Self {
        match s {
            Some(s) if s.is_admin() => {
                Self::Admin(s)
            },
            Some(s) => {
                Self::User(s)
            },
            None => Self::Guest
        }
    }
}

#[allow(non_snake_case)]
fn Ref(title: impl maud::Render, href: &str, active: bool) -> Markup {
    html! {
        span 
            .text-sm.font-medium."space-x-4"
            .text-foreground.transition-colors 
        {
            @if active {
                a."text-foreground/80" 
                href={ (href) } { (title) }
            } @else {
                a."hover:text-foreground/80"."text-foreground/60" 
                href={ (href) } { (title) }
            }
        }
    }
}

#[allow(non_snake_case)]
fn Template(section: Section, auth: Auth, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="es" {
            head {
                title { "Hello, world!" }
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                link href="/style.css" rel="stylesheet";
                script defer src="https://cdn.jsdelivr.net/npm/alpinejs@3.x.x/dist/cdn.min.js" {}
                script {
                    "
                        function toggleDarkMode() {
                            const html = document.querySelector('html');
                            const isDarkMode = html.classList.contains('dark');
                            html.classList.toggle('dark', !isDarkMode);
                            localStorage.setItem('dark', !isDarkMode);

                            return !isDarkMode;
                        }

                        function loadDarkMode() {
                            if (localStorage.getItem('dark') === null) {
                                localStorage.setItem('dark', 'true');
                            }

                            const isDarkMode = localStorage.getItem('dark') === 'true';
                            const html = document.querySelector('html');
                            html.classList.toggle('dark', isDarkMode);

                            return isDarkMode;
                        }

                        loadDarkMode();
                    "
                }
            }

            body.flex.flex-col.min-h-screen.relative
                .bg-background.text-foreground
                x-data="{ 
                    isDark: false,
                    init() {
                        this.isDark = loadDarkMode();
                    }
                }"
            {
                nav
                    .sticky."top-0"."z-50".w-full
                    .flex.flex-row.justify-between.items-center
                    ."px-6"."py-4"
                    ."border-b"."border-zinc-100/95"."dark:border-zinc-800/95"
                    .backdrop-blur
                    ."supports-[backdrop-filter]:bg-background/60"
                {
                    div.flex.flex-row.items-center."space-x-9" {
                        h1 { "Aaa" }
                        div
                            .flex.flex-row.items-center
                            .text-sm.font-medium."space-x-4"
                            .text-foreground.transition-colors 
                        {
                            @if let Auth::Admin(_) = auth {
                                (Ref(Section::Admin, Section::Admin.map_path(), section == Section::Admin))
                            } 

                            @for s in Section::iter() {
                                @if Section::Admin != s {
                                    (Ref(s, s.map_path(), s == section))
                                }
                            }
                        }
                    }

                    div.flex.flex-row.items-center."space-x-4" {
                        @if Auth::Guest == auth {
                            (Ref("Sign in", "/signin", false))
                            
                            (Ref("Sign up", "/signup", false))
                        } @else { 
                            (Ref("Sign out", "/signout", false))
                            
                        }

                        button x-on:click="isDark = toggleDarkMode()" {
                            div."dark:hidden".block {
                                (PreEscaped(include_str!("../static/sun.svg")))
                            }
                            div.hidden."dark:block" {
                                (PreEscaped(include_str!("../static/moon.svg")))
                            }
                        }
                    }
                }

                main { (content) }

                (Footer())
            }
        }
    }
}

#[allow(non_snake_case)]
fn Footer() -> Markup {
    html! {
        footer
            .flex.flex-col.mt-auto 
            .bg-background
        {
            div."px-6"."py-4" {
                p.text-xl.font-bold {
                    "\u{22EF}"
                }
                p.text-xs {
                    "Made with Axum, Maud, Alpine, HTMX & Tailwind."
                }
            }
        }
    }
}