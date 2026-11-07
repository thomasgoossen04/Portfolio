//! Structured site content (projects, skills), embedded from `content/*.toml`
//! (or `examples/content/` when `content/` is absent, see build.rs).
//! User-facing text for these lives in the locale files, keyed by slug / id.

use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Project {
    pub slug: String,
    #[serde(default)]
    pub featured: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    pub repo: Option<String>,
    pub demo: Option<String>,
    /// App store listing (shown as an "App Store" button).
    pub store: Option<String>,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Confident,
    Comfortable,
    Learning,
}

impl Level {
    pub fn key(self) -> &'static str {
        match self {
            Level::Confident => "confident",
            Level::Comfortable => "comfortable",
            Level::Learning => "learning",
        }
    }

    /// Number of filled blocks (out of 3) in the level meter.
    pub fn filled(self) -> usize {
        match self {
            Level::Confident => 3,
            Level::Comfortable => 2,
            Level::Learning => 1,
        }
    }
}

/// A skill is either a plain `name` (shown as-is in every language), or an `id` whose
/// name and level label are translated: `skills.items.<id>.name` / `.level`.
#[derive(Deserialize)]
pub struct Skill {
    pub name: Option<String>,
    pub id: Option<String>,
    pub level: Level,
}

#[derive(Deserialize)]
pub struct Category {
    pub id: String,
    /// Show this category in the home page's "What I work with" block (default: true).
    #[serde(default = "yes")]
    pub home: bool,
    pub items: Vec<Skill>,
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct ProjectFile {
    project: Vec<Project>,
}
#[derive(Deserialize)]
struct SkillFile {
    category: Vec<Category>,
}

struct Content {
    projects: Vec<Project>,
    skills: Vec<Category>,
}

static CONTENT: OnceLock<Content> = OnceLock::new();

/// Parse embedded content; panics on malformed files, so call once at startup.
pub fn load() {
    let projects: ProjectFile = toml::from_str(include_str!(concat!(env!("CONTENT_DIR"), "/projects.toml")))
        .unwrap_or_else(|e| panic!("content/projects.toml: {e}"));
    let skills: SkillFile = toml::from_str(include_str!(concat!(env!("CONTENT_DIR"), "/skills.toml")))
        .unwrap_or_else(|e| panic!("content/skills.toml: {e}"));
    for cat in &skills.category {
        for s in &cat.items {
            if s.name.is_some() == s.id.is_some() {
                panic!("content/skills.toml: each item in `{}` needs exactly one of `name` or `id`", cat.id);
            }
        }
    }
    let _ = CONTENT.set(Content { projects: projects.project, skills: skills.category });
}

fn content() -> &'static Content {
    CONTENT.get().expect("content::load() not called")
}

pub fn projects() -> &'static [Project] {
    &content().projects
}
pub fn project(slug: &str) -> Option<&'static Project> {
    projects().iter().find(|p| p.slug == slug)
}
pub fn skills() -> &'static [Category] {
    &content().skills
}

/// Keys every project/category needs in the locale files; used by the startup check.
pub fn required_locale_keys() -> Vec<String> {
    let mut keys = vec![];
    for p in projects() {
        for f in ["title", "summary", "body"] {
            keys.push(format!("projects.{}.{f}", p.slug));
        }
    }
    for c in skills() {
        keys.push(format!("skills.categories.{}", c.id));
        for id in c.items.iter().filter_map(|s| s.id.as_ref()) {
            keys.push(format!("skills.items.{id}.name"));
            keys.push(format!("skills.items.{id}.level"));
        }
    }
    keys
}
