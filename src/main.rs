use maud::{html, Markup, DOCTYPE, PreEscaped};
use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .fallback_service(ServeDir::new("./static/"));

    axum::serve(TcpListener::bind("[::]:8080").await.unwrap(), app).await.unwrap();
}


async fn index() -> Markup {
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
                            const isDarkMode = localStorage.getItem('dark') === 'true';
                            const html = document.querySelector('html');
                            html.classList.toggle('dark', isDarkMode);

                            return isDarkMode;
                        }
                    "
                }
            }

            body.flex.flex-col.min-h-screen.relative
                .bg-white."dark:bg-zinc-900"
                .text-black."dark:text-gray-200"
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
                    ."p-3"
                    ."border-b-2"."border-zinc-100/95"."dark:border-zinc-800/95"
                    .backdrop-blur
                    ."supports-[backdrop-filter]:bg-white/60"
                    ."supports-[backdrop-filter]:dark:bg-zinc-900/60"
                {
                    div {
                        "Aaa"
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

                div ."h-[1000px]" {
                    p { "Hello, world!" }
                }

                footer
                    .flex.flex-col.mt-auto 
                    ."bg-zinc-100"."dark:bg-black" 
                {
                    div."p-4" {
                        p.text-xl.font-bold {
                            "\u{22EF}"
                        }
                        p.text-xs {
                            "Made with Maud, Alpine, HTMX, Axum, & Tailwind."
                        }
                    }
                }
            }
        }
    }
}
