use auth::Session;
use axum_extra::extract::{PrivateCookieJar, cookie::Cookie};
use axum_server::tls_rustls::RustlsConfig;
use hyper_util::{rt::TokioExecutor, client::legacy::{Client, connect::HttpConnector}};
use maud::{html, Markup, DOCTYPE, PreEscaped};
use axum::{Router, routing::{get, post}, response::{IntoResponse, Redirect}, extract::{State, Host, Path}, Form, http::{StatusCode, Uri}, BoxError, Extension, body::Body};
use state::AppState;
use template::Template;
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
pub mod template;

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
        .route("/signup", get(signup).post(perform_signup))
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
        .route("/get/:id", get(proxy_get_to_middleware))
        .merge(admin)
        .nest("/auth", auth)
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

async fn proxy_get_to_middleware(State(state): State<AppState>, Path((id,)): Path<(String,)>, client: Extension<Client<HttpConnector, Body>>, req: axum::extract::Request) -> impl IntoResponse {    
    let method = req.method().to_owned();
    let (scheme, authority) = state.img_server.split_once("://").unwrap();

    let uri = Uri::builder()
        .scheme(scheme)
        .authority(authority)
        .path_and_query(format!("/{}", id))
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

async fn proxy_upload_to_middleware(State(state): State<AppState>, client: Extension<Client<HttpConnector, Body>>, req: axum::extract::Request) -> impl IntoResponse {
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
struct SignInInfo {
    username: String,
    password: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct SignUpInfo {
    username: String,
    password: String,
    first_name: String,
    last_name: String,
    email: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct User {
    is_admin: Option<bool>,
}

async fn perform_signout(jar: PrivateCookieJar) -> impl IntoResponse {
    let res = (
        jar.remove(Cookie::from("token")),
        StatusCode::OK,
    ).into_response();

    let (mut parts, body) = res.into_parts();

    parts.headers.append("HX-Redirect", "/".parse().unwrap());

    axum::response::Response::from_parts(parts, body)
}

async fn perform_signin(State(state): State<AppState>, jar: PrivateCookieJar, Form(info): Form<SignInInfo>) -> impl IntoResponse {
    let db = state.surreal.get().await.unwrap();
    
    let sign_res = db.signin(Scope {
        namespace: "demo",
        database: "demo",
        scope: "account",
        params: info
    }).await;

    match sign_res {
        Ok(token) => {
            let res = (
                jar.add(
                    Cookie::build(("token", token.as_insecure_token().to_string()))
                    .secure(true)
                    .http_only(true)
                    .path("/")
                    .same_site(axum_extra::extract::cookie::SameSite::Strict)
                    .build()
                ),
                StatusCode::OK,
            ).into_response();

            let (mut parts, body) = res.into_parts();

            parts.headers.append("HX-Redirect", "/".parse().unwrap());
            
            axum::response::Response::from_parts(parts, body)
        },
        Err(e) => {
            println!("Auth error: {:?}", e);
            (StatusCode::UNAUTHORIZED, html! {
                div ."bg-red-100 border border-red-400 text-red-700 px-4 py-2 rounded relative" role="alert" {
                    "Invalid credentials."
                }
            }).into_response()
        }
    }
}

async fn perform_signup(State(state): State<AppState>, jar: PrivateCookieJar, Form(info): Form<SignUpInfo>) -> impl IntoResponse {
    let db = state.surreal.get().await.unwrap();
    
    let sign_res = db.signup(Scope {
        namespace: "demo",
        database: "demo",
        scope: "account",
        params: info
    }).await;

    match sign_res {
        Ok(token) => {
            let res = (
                jar.add(
                    Cookie::build(("token", token.as_insecure_token().to_string()))
                    .secure(true)
                    .http_only(true)
                    .path("/")
                    .same_site(axum_extra::extract::cookie::SameSite::Strict)
                    .build()
                ),
                StatusCode::OK,
            ).into_response();

            let (mut parts, body) = res.into_parts();

            parts.headers.append("HX-Redirect", "/".parse().unwrap());
            
            axum::response::Response::from_parts(parts, body)
        },
        Err(e) => {
            println!("Auth error: {:?}", e);
            (StatusCode::UNAUTHORIZED, html! {
                div ."bg-red-100 border border-red-400 text-red-700 px-4 py-2 rounded relative" role="alert" {
                    "Invalid credentials."
                }
            }).into_response()
        }
    }
}

async fn signup(b: Template) -> Markup {
    b.render(html!{
        div.flex.flex-col.justify-center.h-screen {
            div."flex flex-col items-center" hx-ext="response-targets" {
                form."flex flex-col items-center space-y-4 border border-zinc-100/95 dark:border-zinc-800/95 p-4 rounded-md" {
                    h1."text-4xl".font-bold {
                        "Sign up"
                    }
                    div #err {}
                    input."rounded-md border border-zinc-100/95 dark:border-zinc-800/95 p-2".text-black name="username" type="text" placeholder="Username" {}
                    input."rounded-md border border-zinc-100/95 dark:border-zinc-800/95 p-2".text-black name="password" type="password" placeholder="Password" {}
                    
                    input."rounded-md border border-zinc-100/95 dark:border-zinc-800/95 p-2".text-black name="first_name" type="text" placeholder="First name" {}
                    input."rounded-md border border-zinc-100/95 dark:border-zinc-800/95 p-2".text-black name="last_name" type="text" placeholder="Last name" {}

                    input."rounded-md border border-zinc-100/95 dark:border-zinc-800/95 p-2".text-black name="email" type="email" placeholder="Email" {}

                    button."rounded-md border border-zinc-100/95 dark:border-zinc-800/95 p-2".w-full 
                    hx-post="/auth/signup" "hx-target-401"="#err"
                    {
                        "Sign up"
                    }
                    
                }
            }
        }
    })  
}

async fn signin(b: Template) -> Markup {
    b.render(html!{
        div.flex.flex-col.justify-center.h-screen {
            div."flex flex-col items-center" hx-ext="response-targets" {
                form."flex flex-col items-center space-y-4 border border-zinc-100/95 dark:border-zinc-800/95 p-4 rounded-md" {
                    h1."text-4xl".font-bold {
                        "Sign in"
                    }
                    div #err {}
                    input."rounded-md border border-zinc-100/95 dark:border-zinc-800/95 p-2".text-black name="username" type="text" placeholder="Username" {}
                    input."rounded-md border border-zinc-100/95 dark:border-zinc-800/95 p-2".text-black name="password" type="password" placeholder="Password" {}
                    button."rounded-md border border-zinc-100/95 dark:border-zinc-800/95 p-2".w-full 
                    hx-post="/auth/signin" "hx-target-401"="#err"
                    {
                        "Sign in"
                    }
                    
                }
            }
        }
    })  
}

async fn admin(b: Template, session: Session) -> Markup {
    b.render(html!{
        div."p-4".flex.flex-col {
            h1."text-4xl".font-bold { "Hola, " (session.first_name()) "!" }
        }
    })  
}

async fn root(b: Template) -> Markup {
    b.render(html!{
        h1."text-4xl".font-bold ."h-[1000px]" {
            "Hello, world!"
        }
    })  
}

async fn other(b: Template) -> Markup {
    b.render(html!{
        h1."text-4xl".font-bold ."h-[1000px]" {
            "Other!"
        }
    })  
}

async fn about(b: Template) -> Markup {
    b.render(html!{
        h1."text-4xl".font-bold ."h-[1000px]" {
            "About!"
        }
    })  
}

