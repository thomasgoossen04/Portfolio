//! Translations. Strings live in `locales/<lang>.toml` (embedded at compile time),
//! nested by section and referenced as `i18n.t("home.title")`.
//! `{name}` placeholders are filled via `i18n.t_with("key", &[("name", "value")])`.

use std::collections::HashMap;
use std::sync::OnceLock;

use rocket::http::{Cookie, CookieJar, SameSite};
use rocket::request::{FromRequest, Outcome, Request};

pub const LANG_COOKIE: &str = "lang";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    Nl,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::En, Lang::Nl];

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Nl => "nl",
        }
    }

    pub fn parse(s: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|l| l.code().eq_ignore_ascii_case(s.trim()))
    }

    fn source(self) -> &'static str {
        match self {
            Lang::En => include_str!("../locales/en.toml"),
            Lang::Nl => include_str!("../locales/nl.toml"),
        }
    }

    /// Pick the best language from an Accept-Language header value.
    fn from_accept_language(header: &str) -> Option<Lang> {
        header
            .split(',')
            .filter_map(|part| Lang::parse(part.split(';').next()?.split('-').next()?))
            .next()
    }
}

type Flat = HashMap<String, String>;

fn flatten(prefix: &str, table: &toml::Table, out: &mut Flat) {
    for (k, v) in table {
        let key = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
        match v {
            toml::Value::String(s) => {
                out.insert(key, s.clone());
            }
            toml::Value::Table(t) => flatten(&key, t, out),
            other => panic!("locale key `{key}` must be a string or table, got {other}"),
        }
    }
}

struct Store(HashMap<&'static str, Flat>);

static STORE: OnceLock<Store> = OnceLock::new();

/// Parse all locale files and verify they define the same keys. Panics on problems,
/// so call it once at startup.
pub fn load() {
    let mut map = HashMap::new();
    for lang in Lang::ALL {
        let table: toml::Table = lang
            .source()
            .parse()
            .unwrap_or_else(|e| panic!("locales/{}.toml: {e}", lang.code()));
        let mut flat = Flat::new();
        flatten("", &table, &mut flat);
        map.insert(lang.code(), flat);
    }

    let reference = &map[Lang::En.code()];
    for lang in Lang::ALL {
        let flat = &map[lang.code()];
        let mut missing: Vec<_> = reference.keys().filter(|k| !flat.contains_key(*k)).collect();
        let mut extra: Vec<_> = flat.keys().filter(|k| !reference.contains_key(*k)).collect();
        missing.sort();
        extra.sort();
        if !missing.is_empty() || !extra.is_empty() {
            panic!(
                "locales/{}.toml is out of sync with en.toml\n  missing: {missing:?}\n  extra: {extra:?}",
                lang.code()
            );
        }
    }
    // Keys required by structured content (projects, skills) must exist in every language.
    for key in crate::content::required_locale_keys() {
        for lang in Lang::ALL {
            if !map[lang.code()].contains_key(&key) {
                panic!("locales/{}.toml is missing `{key}` (required by content/*.toml)", lang.code());
            }
        }
    }
    let _ = STORE.set(Store(map));
}

/// Per-request translator. Also usable as a Rocket request guard.
#[derive(Clone, Copy)]
pub struct I18n {
    pub lang: Lang,
}

impl I18n {
    pub fn t(&self, key: &str) -> String {
        match STORE.get().and_then(|s| s.0[self.lang.code()].get(key)) {
            Some(s) => s.clone(),
            None => {
                eprintln!("missing translation key `{key}` ({})", self.lang.code());
                format!("[{key}]")
            }
        }
    }

    pub fn t_with(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut s = self.t(key);
        for (name, value) in args {
            s = s.replace(&format!("{{{name}}}"), value);
        }
        s
    }
}

impl I18n {
    /// Priority: language cookie > Accept-Language header > English.
    pub fn from_request_sync(req: &Request<'_>) -> I18n {
        let lang = req
            .cookies()
            .get(LANG_COOKIE)
            .and_then(|c| Lang::parse(c.value()))
            .or_else(|| {
                req.headers().get_one("Accept-Language").and_then(Lang::from_accept_language)
            })
            .unwrap_or(Lang::En);
        I18n { lang }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for I18n {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(I18n::from_request_sync(req))
    }
}

pub fn set_lang_cookie(jar: &CookieJar<'_>, lang: Lang) {
    jar.add(
        Cookie::build((LANG_COOKIE, lang.code()))
            .path("/")
            .same_site(SameSite::Lax)
            .max_age(rocket::time::Duration::days(365)),
    );
}
