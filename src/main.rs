use auth::Session;
use axum_extra::extract::{PrivateCookieJar, cookie::Cookie};
use maud::{html, Markup, DOCTYPE, PreEscaped};
use axum::{Router, routing::get, response::{IntoResponse, Redirect, Response}, extract::{State, Request}, Form, middleware::{self, Next}, http::StatusCode};
use state::AppState;
use strum::{EnumIter, IntoEnumIterator};
use surrealdb::opt::auth::Scope;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

pub mod pool;
pub mod auth;
pub mod state;
pub mod error;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let host = std::env::var("HOST").expect("HOST must be set");
    let surreal = std::env::var("SURREAL").expect("SURREAL must be set");
    let s_size = std::env::var("POOL_SIZE").expect("POOL_SIZE must be set");

    let state = state::AppState::new(pool::SPool::new(surreal.as_str(), s_size.parse::<usize>().unwrap()));

    let auth : Router<AppState> = Router::new()
        .route("/signin", get(signin).post(perform_signin))
        .route("/signup", get(signin))
        .route_layer(middleware::from_fn_with_state(state.clone(), middleware_redirect_already_logged_in));


    let app = Router::new()
        .route("/", get(root))
        .route("/other", get(other))
        .route("/admin", get(admin))
        .merge(auth)
        .fallback_service(ServeDir::new("./static/"))
        .layer(tower_http::compression::CompressionLayer::new())
        .with_state(state);
        
 

    axum::serve(TcpListener::bind(host).await.unwrap(), app).await.unwrap();
}

async fn middleware_redirect_already_logged_in(_: State<AppState>, session: Result<Session, error::Error>, req: Request, next: Next) -> Response {
    if session.is_ok() {
        return Redirect::to("/").into_response();
    }
    
    next.run(req).await
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


async fn perform_signin(State(state): State<AppState>, jar: PrivateCookieJar, Form(info): Form<UserInfo>) -> impl IntoResponse {
    let db = state.surreal.get().await.unwrap();
    let username = info.username.clone();
    
    let sign_res = db.signin(Scope {
        namespace: "demo",
        database: "demo",
        scope: "account",
        params: info
    }).await;

    match sign_res {
        Ok(token) => {
            let is_admin: User = db.select(("user", &username)).await.unwrap().unwrap();

            let jar = jar.add(Cookie::new("token", token.as_insecure_token().to_string()));
            let jar = jar.add(Cookie::new("user_id", format!("user:{}", username)));
            let jar = jar.add(Cookie::new("is_admin", is_admin.is_admin.unwrap_or_default().to_string()));

            (
                jar,
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
    Template(Section::Home, Auth::from(session), html!{
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

async fn admin(session: Option<Session>) -> Markup {
    Template(Section::Admin, Auth::from(session), html!{
        h1."text-4xl".font-bold ."h-[1000px]" {
            "Admin!"
        }
    })  
}

async fn root(session: Option<Session>) -> Markup {
    Template(Section::Home, Auth::from(session), html!{
        h1."text-4xl".font-bold ."h-[1000px]" {
            "Hello, world!"
        }
    })  
}

async fn other(session: Option<Session>) -> Markup {
    Template(Section::Other, Auth::from(session), html!{
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Auth {
    User,
    Admin,
    Guest,
}


impl From<bool> for Auth {
    fn from(b: bool) -> Self {
        if b {
            Self::Admin
        } else {
            Self::User
        }
    }
}

impl From<Option<Session>> for Auth {
    fn from(s: Option<Session>) -> Self {
        if s.is_none() {
            Self::Guest
        } else {
            Self::from(Session::is_some_admin(s))
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
                script src="https://cdn.jsdelivr.net/npm/alpinejs@3.x.x/dist/cdn.min.js" {}
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
                            @if let Auth::Admin = auth {
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
                        }

                        button x-on:click="isDark = toggleDarkMode()" {
                            div."dark:hidden block" {
                                (PreEscaped(include_str!("../static/sun.svg")))
                            }
                            div."hidden dark:block" {
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