//! Reusable view fragments shared between pages.

use maud::{Markup, html};

use crate::content::{self, Level, Project};
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
    let cats: Vec<_> = content::skills().iter().filter(|c| !home || c.home).collect();
    // With 3+ cards, the longest one spans two rows and the others stack beside it,
    // so one long list doesn't leave the other cards mostly empty.
    let tall = (cats.len() >= 3)
        .then(|| cats.iter().enumerate().max_by_key(|(_, c)| c.items.len()).map(|(i, _)| i))
        .flatten();
    html! {
        div.grid.grid-skills.has-tall[tall.is_some()] {
            @for (idx, cat) in cats.iter().enumerate() {
                div.card.tall[tall == Some(idx)] {
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
                                @if let Some(icon) = &s.icon {
                                    span.skill-icon aria-hidden="true"
                                        style={ "--icon: url('/static/icons/" (icon) ".svg')" } {}
                                }
                                span.skill-name title=(name) { (name) }
                                span.skill-level {
                                    @if cat.labels {
                                        span.meter aria-hidden="true" { (meter_blocks(s.level)) }
                                        span.dim { (level) }
                                    } @else {
                                        span.meter role="img" aria-label=(level) title=(level) {
                                            (meter_blocks(s.level))
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Meter blocks for a level; "learning" draws its filled block striped (in progress).
fn meter_blocks(level: Level) -> Markup {
    let wip = level == Level::Learning;
    html! {
        @for i in 0..3 {
            span.block.on[i < level.filled()].wip[wip && i < level.filled()] {}
        }
    }
}

/// One-line key explaining the meter, for cards that hide the level labels.
pub fn skills_legend(i18n: I18n) -> Markup {
    html! {
        div.row.legend.dim {
            span { (i18n.t("skills.legend")) ":" }
            @for level in Level::ALL {
                span.legend-item {
                    span.meter aria-hidden="true" { (meter_blocks(level)) }
                    (i18n.t(&format!("skills.levels.{}", level.key())))
                }
            }
        }
    }
}

/// Education timeline (about and CV pages); text comes from `about.timeline.*`.
pub fn education_timeline(i18n: I18n) -> Markup {
    html! {
        ol.timeline {
            @for key in ["bsc", "exchange", "vwo"] {
                li class={ "timeline-item" @if key == "exchange" { " nested" } } {
                    span.dim { (i18n.t(&format!("about.timeline.{key}.period"))) }
                    h3 { (i18n.t(&format!("about.timeline.{key}.title"))) }
                    p { (i18n.t(&format!("about.timeline.{key}.place"))) }
                }
            }
        }
    }
}
