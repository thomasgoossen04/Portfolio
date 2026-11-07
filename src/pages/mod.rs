use maud::{Markup, html};
use rocket::Route;

use crate::content;
use crate::i18n::I18n;
use crate::site;
use crate::theme;
use crate::ui;

pub fn routes() -> Vec<Route> {
    routes![index, projects, project_detail, skills, about, cv, contact]
}

#[get("/")]
fn index(i18n: I18n) -> Markup {
    theme::page(i18n, &i18n.t("home.page_title"), html! {
        div.stack {
            section.hero {
                p.dim { "$ whoami" }
                h1 { (site::name()) }
                p.role { (i18n.t("home.role")) }
                p { (i18n.t("home.tagline")) }
                div.row {
                    a.btn href="/projects" { (i18n.t("home.cta_projects")) }
                    a.btn.ghost href="/contact" { (i18n.t("home.cta_contact")) }
                }
            }
            section.stack {
                h2 { (i18n.t("home.featured")) }
                div.grid {
                    @for p in content::projects().iter().filter(|p| p.featured) {
                        (ui::project_card(i18n, p))
                    }
                }
                a href="/projects" { (i18n.t("common.view_all_projects")) " →" }
            }
            section.stack {
                h2 { (i18n.t("home.skills_heading")) }
                (ui::skills_block(i18n))
                a href="/skills" { (i18n.t("common.view_all_skills")) " →" }
            }
        }
    })
}

#[get("/projects")]
fn projects(i18n: I18n) -> Markup {
    theme::page(i18n, &i18n.t("projects.page_title"), html! {
        div.stack {
            h1 { (i18n.t("projects.heading")) }
            p.dim { (i18n.t("projects.intro")) }
            div.grid {
                @for p in content::projects() { (ui::project_card(i18n, p)) }
            }
        }
    })
}

#[get("/projects/<slug>")]
fn project_detail(i18n: I18n, slug: &str) -> Option<Markup> {
    let p = content::project(slug)?;
    let k = |f: &str| i18n.t(&format!("projects.{}.{f}", p.slug));
    Some(theme::page(i18n, &k("title"), html! {
        div.stack {
            a href="/projects" { (i18n.t("common.back_to_projects")) }
            h1 { (k("title")) }
            div.row { @for tag in &p.tags { span.tag { (tag) } } }
            p { (k("body")) }
            div.row {
                @if let Some(url) = &p.repo { a.btn.ghost href=(url) { (i18n.t("common.source")) } }
                @if let Some(url) = &p.demo { a.btn href=(url) { (i18n.t("common.demo")) } }
            }
        }
    }))
}

#[get("/skills")]
fn skills(i18n: I18n) -> Markup {
    theme::page(i18n, &i18n.t("skills.page_title"), html! {
        div.stack {
            h1 { (i18n.t("skills.heading")) }
            p.dim { (i18n.t("skills.intro")) }
            (ui::skills_block(i18n))
        }
    })
}

#[get("/about")]
fn about(i18n: I18n) -> Markup {
    theme::page(i18n, &i18n.t("about.page_title"), html! {
        div.stack {
            h1 { (i18n.t("about.heading")) }
            p { (i18n.t("about.intro")) }
            h2 { (i18n.t("about.study_heading")) }
            p { (i18n.t("about.study")) }
            h2 { (i18n.t("about.interests_heading")) }
            p { (i18n.t("about.interests")) }
        }
    })
}

#[get("/cv")]
fn cv(i18n: I18n) -> Markup {
    theme::page(i18n, &i18n.t("cv.page_title"), html! {
        div.stack {
            h1 { (i18n.t("cv.heading")) }
            // TODO: drop a PDF in static/ and link it here.
            h2 { (i18n.t("cv.education")) }
            div.card { (i18n.t("cv.education_item")) }
            h2 { (i18n.t("cv.experience")) }
            div.card { (i18n.t("cv.experience_item")) }
        }
    })
}

#[get("/contact")]
fn contact(i18n: I18n) -> Markup {
    theme::page(i18n, &i18n.t("contact.page_title"), html! {
        div.stack {
            h1 { (i18n.t("contact.heading")) }
            p { (i18n.t("contact.intro")) }
            ul.links {
                li { span.dim { (i18n.t("contact.email")) ": " } a href={ "mailto:" (site::email()) } { (site::email()) } }
                li { span.dim { (i18n.t("contact.github")) ": " } a href=(site::github()) { (site::github()) } }
                li { span.dim { (i18n.t("contact.linkedin")) ": " } a href=(site::linkedin()) { (site::linkedin()) } }
            }
        }
    })
}

#[catch(404)]
pub fn not_found(req: &rocket::Request<'_>) -> Markup {
    // Catchers can't use request guards directly; build the translator by hand.
    let i18n = I18n::from_request_sync(req);
    theme::page(i18n, &i18n.t("not_found.page_title"), html! {
        div.stack {
            h1 { (i18n.t("not_found.heading")) }
            p { (i18n.t("not_found.text")) }
            a.btn href="/" { (i18n.t("not_found.back")) }
        }
    })
}
