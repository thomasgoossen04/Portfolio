//! Reusable view fragments shared between pages.

use maud::{Markup, html};

use crate::content::{self, Project};
use crate::i18n::I18n;

pub fn project_card(i18n: I18n, p: &Project) -> Markup {
    let k = |f: &str| i18n.t(&format!("projects.{}.{f}", p.slug));
    html! {
        a.card href={ "/projects/" (p.slug) } {
            h3 { (k("title")) }
            p.dim { (k("summary")) }
            div.row.tags {
                @for tag in &p.tags { span.tag { (tag) } }
            }
        }
    }
}

/// Skill categories with level meters; `home` limits it to categories marked for the home page.
pub fn skills_block(i18n: I18n, home: bool) -> Markup {
    html! {
        div.grid.grid-skills {
            @for cat in content::skills().iter().filter(|c| !home || c.home) {
                div.card {
                    h3 { (i18n.t(&format!("skills.categories.{}", cat.id))) }
                    ul.skills {
                        @for s in &cat.items {
                            @let (name, level) = match &s.id {
                                Some(id) => (
                                    i18n.t(&format!("skills.items.{id}.name")),
                                    i18n.t(&format!("skills.items.{id}.level")),
                                ),
                                None => (
                                    s.name.clone().unwrap_or_default(),
                                    i18n.t(&format!("skills.levels.{}", s.level.key())),
                                ),
                            };
                            li {
                                span.skill-name title=(name) { (name) }
                                span.skill-level {
                                    span.meter aria-hidden="true" {
                                        @for i in 0..3 {
                                            span.block.on[i < s.level.filled()] {}
                                        }
                                    }
                                    span.dim { (level) }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
