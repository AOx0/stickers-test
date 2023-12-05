use maud::{html, Markup, DOCTYPE, PreEscaped};
use axum::{Router, routing::get};
use strum::{EnumIter, IntoEnumIterator};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(root))
        .route("/other", get(other))
        .fallback_service(ServeDir::new("./static/"));

    axum::serve(TcpListener::bind("[::]:8080").await.unwrap(), app).await.unwrap();
}

async fn root() -> Markup {
    Template(Section::Home, html!{
        h1."text-4xl".font-bold ."h-[1000px]" {
            "Hello, world!"
        }
    })  
}

async fn other() -> Markup {
    Template(Section::Other, html!{
        h1."text-4xl".font-bold ."h-[1000px]" {
            "Other!"
        }
    })  
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
enum Section {
    Home,
    Other,
}

impl Section {
    fn map_path(&self) -> &'static str {
        match self {
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

#[allow(non_snake_case)]
fn Template(section: Section, content: Markup) -> Markup {
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
                            @for s in Section::iter() {
                                @if s == section {
                                    a."text-foreground/80" 
                                    href={ (s.map_path()) } { (s) }
                                } @else {
                                    a."hover:text-foreground/80"."text-foreground/60" 
                                    href={ (s.map_path()) } { (s) }
                                }
                            }
                        }
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