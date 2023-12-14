use auth::Session;
use axum_extra::extract::{PrivateCookieJar, cookie::Cookie};
use axum_server::tls_rustls::RustlsConfig;
use hyper_util::{rt::TokioExecutor, client::legacy::{Client, connect::HttpConnector}};
use maud::{html, Markup, DOCTYPE, PreEscaped};
use axum::{Router, routing::{get, post}, response::{IntoResponse, Redirect}, extract::{State, Host}, Form, http::{StatusCode, Uri}, BoxError, Extension, body::Body};
use state::AppState;
use tower_http::add_extension::AddExtensionLayer;
use strum::{EnumIter, IntoEnumIterator};
use surrealdb::opt::auth::Scope;
use axum::handler::HandlerWithoutStateExt;
use tower_http::services::ServeDir;

pub mod pool;
pub mod auth;
pub mod state;
pub mod error;
pub mod middleware;

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
    let img_server = std::env::var("IMG_SERVER").expect("IMG_SERVER must be set");

    let surreal = pool::SPool::new(surreal.as_str(), s_size.parse::<usize>().unwrap());
    let state = state::AppState::new(surreal, &img_server);

    let ports = Ports {
        http: 80,
        https: 443,
    };

    let config = RustlsConfig::from_pem_file(
        "/Users/alejandro/Downloads/OsornioLOL/certificate.crt",
        "/Users/alejandro/Downloads/OsornioLOL/private.key",
    )
    .await
    .unwrap();

    let client = hyper_util::client::legacy::Client::builder(TokioExecutor::new())
    .http2_only(true)
    .build_http::<Body>();

    tokio::spawn(redirect_http_to_https(ports));

    let auth : Router<AppState> = Router::new()
        .route("/signin", get(signin).post(perform_signin))
        .route("/signup", get(signin))
        .route_layer(middleware::from_fn_with_state(state.clone(), middleware::redirect_already_logged_in));

    let admin : Router<AppState> = Router::new()
        .route("/admin", get(admin))
        .route_layer(middleware::from_fn_with_state(state.clone(), middleware::assert_is_admin));

    let app = Router::new()
        .route("/", get(root))
        .route("/other", get(other))
        .route("/signout", get(perform_signout))
        .route("/about", get(about))
        .route("/upload", post(proxy_upload_to_middleware))
        .merge(admin)
        .merge(auth)
        .fallback_service(ServeDir::new("./static/"))
        .layer(tower_http::compression::CompressionLayer::new())
        .route_layer(middleware::from_fn(middleware::insert_securiy_headers))
        .layer(AddExtensionLayer::new(client))
        .with_state(state);
        
    axum_server::bind_rustls(format!("[::]:{}", ports.https).parse().unwrap(), config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn proxy_upload_to_middleware(State(state): State<AppState>, client: Extension<Client<HttpConnector, Body>>, req: axum::extract::Request) -> impl IntoResponse {
    use hyper_util::client::legacy::Client;
    
    let method = req.method().to_owned();
    let (scheme, authority) = state.img_server.split_once("://").unwrap();

    let uri = Uri::builder()
        .scheme(scheme)
        .authority(authority)
        .path_and_query("/new")
        .build()
        .unwrap();

    let headers = req.headers().to_owned();
    let body = req.into_body();

    let mut req = hyper::Request::builder()
        .method(method)
        .uri(uri)
        .body(body)
        .unwrap();

    *req.headers_mut() = headers;


    let res = client.request(req).await.unwrap();

    res.into_response()
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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct UserInfo {
    username: String,
    password: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct User {
    is_admin: Option<bool>,
}

async fn perform_signout(jar: PrivateCookieJar) -> impl IntoResponse {
    (
        jar.remove(Cookie::from("token")),
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

async fn about(session: Option<Session>) -> Markup {
    Template(Section::Other, Auth::from(session.as_ref()), html!{
        h1."text-4xl".font-bold ."h-[1000px]" {
            "About!"
        }
    })  
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
enum Section {
    Admin,
    Home,
    About,
    Other,
}

impl Section {
    fn map_path(&self) -> &'static str {
        match self {
            Self::Admin => "/admin",
            Self::Home => "/",
            Self::About => "/about",
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
                    ."h-[65px]"
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

                    div
                        .flex.flex-row.items-center."space-x-4"
                        x-data = "{ open: false }"
                    {
                        button x-on:click="isDark = toggleDarkMode()" {
                            div."dark:hidden".block."hover:opacity-80".transition-opacity {
                                (PreEscaped(include_str!("../static/sun.svg")))
                            }
                            div.hidden."dark:block"."hover:opacity-80".transition-opacity {
                                (PreEscaped(include_str!("../static/moon.svg")))
                            }
                        }

                        @match auth {
                            Auth::Guest => {
                                (Ref("Sign in", "/signin", false))
                                (Ref("Sign up", "/signup", false))
                            }
                            Auth::User(s) | Auth::Admin(s) => {
                                div 
                                    .rounded-full.inline-block."p-2".select-none
                                    ."bg-zinc-100/95"."dark:bg-zinc-800/95"
                                    ."hover:opacity-80".transition-opacity
                                    x-on:click="open = !open"
                                {
                                    p 
                                        ."text-foreground/80".text-xs
                                        .font-bold."hover:opacity-100"
                                    {
                                        (s.first_name()[..1].to_uppercase())
                                        (s.last_name()[..1].to_uppercase())
                                    }
                                }

                                div 
                                    .absolute
                                    .shadow-md.rounded-xl.bg-background."z-50"
                                    ."top-0"."right-0"
                                    ."px-6"."py-4"
                                    .hidden
                                    x-show="open"
                                    x-init="$el.classList.remove('hidden')"
                                    x-transition
                                {
                                    div."flex flex-col space-y-2"."p-2".flex.flex-col {

                                        div.flex.flex-row.justify-center."space-x-4" {
                                            p.text-lg.font-bold {
                                                (s.first_name()) " " (s.last_name())
                                            }

                                            button x-on:click="open = !open" {
                                                (PreEscaped(include_str!("../static/close.svg")))
                                            }
                                        }
                                        

                                        hr."opacity-70";
                                        
                                        (Ref("Sign out", "/signout", false))
                                    }
                                }
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