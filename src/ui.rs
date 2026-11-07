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

/// All skill categories with level meters.
pub fn skills_block(i18n: I18n) -> Markup {
    html! {
        div.grid.grid-skills {
            @for cat in content::skills() {
                div.card {
                    h3 { (i18n.t(&format!("skills.categories.{}", cat.id))) }
                    ul.skills {
                        @for s in &cat.items {
                            @let level = i18n.t(&format!("skills.levels.{}", s.level.key()));
                            li {
                                span.skill-name title=(s.name) { (s.name) }
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
