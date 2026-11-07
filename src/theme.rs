//! Site-wide layout. All styling lives in `static/theme.css`.

use maud::{DOCTYPE, Markup, html};

use crate::i18n::{I18n, Lang};

/// Wrap page content in the shared document shell (head, theme, nav, footer).
pub fn page(i18n: I18n, title: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang=(i18n.lang.code()) {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " · " (i18n.t("meta.site_title")) }
                link rel="stylesheet" href="/static/theme.css";
                @if cfg!(debug_assertions) { (editor_head()) }
            }
            body {
                nav.nav {
                    div.container.row {
                        a href="/" { (i18n.t("nav.home")) }
                        @for (href, key) in [("/projects", "nav.projects"), ("/skills", "nav.skills"), ("/about", "nav.about"), ("/cv", "nav.cv"), ("/contact", "nav.contact")] {
                            a href=(href) { (i18n.t(key)) }
                        }
                        span.dim.push-right { (i18n.t("nav.language")) ":" }
                        @for lang in Lang::ALL {
                            @if lang == i18n.lang {
                                span.lang-current aria-current="true" { (lang.code().to_uppercase()) }
                            } @else {
                                a.lang-other href={ "/lang/" (lang.code()) } { (lang.code().to_uppercase()) }
                            }
                        }
                    }
                }
                main.container { (content) }
                footer.footer {
                    div.container {
                        (i18n.t_with("footer.copyright", &[("year", "2026"), ("name", crate::site::name())]))
                    }
                }
            }
        }
    }
}

#[cfg(debug_assertions)]
fn editor_head() -> Markup {
    crate::editor::head()
}
#[cfg(not(debug_assertions))]
fn editor_head() -> Markup {
    html! {}
}
