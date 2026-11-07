//! Personal details (name, links), embedded from `content/site.toml`.

use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Deserialize)]
struct Site {
    name: String,
    email: String,
    github: String,
    linkedin: String,
}

static SITE: OnceLock<Site> = OnceLock::new();

/// Parse embedded site details; panics on a malformed file, so call once at startup.
pub fn load() {
    let site: Site = toml::from_str(include_str!(concat!(env!("CONTENT_DIR"), "/site.toml")))
        .unwrap_or_else(|e| panic!("content/site.toml: {e}"));
    let _ = SITE.set(site);
}

fn site() -> &'static Site {
    SITE.get().expect("site::load() not called")
}

pub fn name() -> &'static str {
    &site().name
}
pub fn email() -> &'static str {
    &site().email
}
pub fn github() -> &'static str {
    &site().github
}
pub fn linkedin() -> &'static str {
    &site().linkedin
}
