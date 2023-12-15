use axum::{extract::FromRequestParts, RequestPartsExt, async_trait};
use http::request::Parts;
use maud::{Markup, html, DOCTYPE, PreEscaped};
use strum::{EnumIter, IntoEnumIterator};

use crate::{auth::Session, state::AppState, error::Error};

#[derive(Debug, Clone, Copy)]
pub enum ContentMode {
    Full,
    Embedded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Auth {
    User(Session),
    Admin(Session),
    Guest,
}

pub struct Template {
    mode: ContentMode,
    auth: Auth,
}


impl Template {
    pub fn mode(&self) -> ContentMode {
        self.mode
    }

    pub fn auth(&self) -> &Auth {
        &self.auth
    }

    pub fn is_admin(&self) -> bool {
        match self.auth {
            Auth::Admin(_) => true,
            _ => false,
        }
    }

    pub fn render(self, content: Markup) -> Markup {
        match self.mode {
            ContentMode::Full => {
                Template(self.auth, ContentMode::Full, content)
            }
            ContentMode::Embedded => {
                html! {
                    (content)
                }
                
            }  
        }
    }
}

impl From<Option<Session>> for Auth {
    fn from(s: Option<Session>) -> Self {
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

#[async_trait]
impl FromRequestParts<AppState> for Template {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        if parts.headers.get("HX-Request").is_some() {
            Ok(Template {
                mode: ContentMode::Embedded,
                auth: Auth::Guest,
            })
        } else {
            Ok(Template {
                mode: ContentMode::Full,
                auth: Auth::from(parts.extract_with_state::<Option<Session>, AppState>(state).await.map_err(|e| {
                    println!("Auth error: {:?}", e);
                    Error::AuthFailed
                })?),
            })
        }
    }
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



#[allow(non_snake_case)]
fn Ref(title: impl maud::Render, href: &str, active: bool) -> Markup {
    html! {
        span 
            .text-sm.font-medium."space-x-4"
            .text-foreground.transition-colors 
        {
            p."hover:text-foreground/80"."text-foreground/60" 
            hx-boost="true"
            hx-push-url="true"
            hx-target="#main"
            hx-get={ (href) } { (title) } 
        }
    }
}

#[allow(non_snake_case)]
fn Template(auth: Auth, mode: ContentMode, content: Markup) -> Markup {
    if let ContentMode::Embedded = mode {
        return html! {
            (content)
        }
    }

    html! {
        (DOCTYPE)
        html lang="es" {
            head {
                title { "Hello, world!" }
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                link href="/style.css" rel="stylesheet";
                script defer src="https://cdn.jsdelivr.net/npm/alpinejs@3.x.x/dist/cdn.min.js" {}
                script src="https://unpkg.com/htmx.org@1.9.9" {}
                script src="https://unpkg.com/htmx.org/dist/ext/response-targets.js" {}
                script src="https://unpkg.com/htmx.org/dist/ext/head-support.js" {}
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

            body
                hx-ext="head-support"
                
                .flex.flex-col.min-h-screen.relative
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
                        
                        h1.font-semibold { "AOx0" }
                        
                        div
                            .flex.flex-row.items-center
                            .text-sm.font-medium."space-x-4"
                            .text-foreground.transition-colors 
                        {
                            @if let Auth::Admin(_) = auth {
                                (Ref(Section::Admin, Section::Admin.map_path(), false))
                            } 

                            @for s in Section::iter() {
                                @if Section::Admin != s {
                                    (Ref(s, s.map_path(), false))
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
                                (Ref("Sign in", "/auth/signin", false))
                                (Ref("Sign up", "/auth/signup", false))
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

                main #main { (content) }

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