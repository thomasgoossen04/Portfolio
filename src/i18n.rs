//! Translations. Strings live in `locales/<lang>.toml` (embedded at compile time),
//! nested by section and referenced as `i18n.t("home.title")`.
//! `{name}` placeholders are filled via `i18n.t_with("key", &[("name", "value")])`.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

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

    /// UI strings (public, `locales/`).
    fn source(self) -> &'static str {
        match self {
            Lang::En => include_str!("../locales/en.toml"),
            Lang::Nl => include_str!("../locales/nl.toml"),
        }
    }

    /// Personal copy (`content/locales/`, see build.rs), merged on top of the UI strings.
    fn content_source(self) -> &'static str {
        match self {
            Lang::En => include_str!(concat!(env!("CONTENT_DIR"), "/locales/en.toml")),
            Lang::Nl => include_str!(concat!(env!("CONTENT_DIR"), "/locales/nl.toml")),
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

fn flatten(prefix: &str, table: &toml::Table, out: &mut Flat) -> Result<(), String> {
    for (k, v) in table {
        let key = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
        match v {
            toml::Value::String(s) => {
                out.insert(key, s.clone());
            }
            toml::Value::Table(t) => flatten(&key, t, out)?,
            other => return Err(format!("locale key `{key}` must be a string or table, got {other}")),
        }
    }
    Ok(())
}

struct Store(HashMap<&'static str, Flat>);

/// A `RwLock` so the dev editor can swap in freshly saved strings without a restart.
static STORE: OnceLock<RwLock<Store>> = OnceLock::new();

/// Parse all locale files and verify they define the same keys. Panics on problems,
/// so call it once at startup.
pub fn load() {
    let store = build(|lang| (lang.source().to_string(), lang.content_source().to_string()))
        .unwrap_or_else(|e| panic!("{e}"));
    let _ = STORE.set(RwLock::new(store));
}

/// Re-read the locale files from disk (dev editor only). On error the old strings stay.
#[cfg(debug_assertions)]
pub fn reload_from_disk() -> Result<(), String> {
    let mut files = HashMap::new();
    for lang in Lang::ALL {
        let read = |path: String| std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"));
        let ui = read(locale_path(LocaleFile::Ui, lang))?;
        let content = read(locale_path(LocaleFile::Content, lang))?;
        files.insert(lang.code(), (ui, content));
    }
    let store = build(|lang| files[lang.code()].clone())?;
    *STORE.get().expect("i18n::load() not called").write().unwrap() = store;
    Ok(())
}

/// The two kinds of locale file: public UI strings and personal copy.
#[cfg(debug_assertions)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LocaleFile {
    Ui,
    Content,
}

/// On-disk path of a locale file (absolute, fixed at compile time; fine for dev).
#[cfg(debug_assertions)]
pub fn locale_path(file: LocaleFile, lang: Lang) -> String {
    match file {
        LocaleFile::Ui => format!("{}/locales/{}.toml", env!("CARGO_MANIFEST_DIR"), lang.code()),
        LocaleFile::Content => format!("{}/locales/{}.toml", env!("CONTENT_DIR"), lang.code()),
    }
}

/// `sources(lang)` returns the (UI strings, personal copy) TOML text for that language.
fn build(sources: impl Fn(Lang) -> (String, String)) -> Result<Store, String> {
    let mut map = HashMap::new();
    for lang in Lang::ALL {
        let code = lang.code();
        let (ui, content) = sources(lang);
        let table: toml::Table = ui.parse().map_err(|e| format!("locales/{code}.toml: {e}"))?;
        let mut flat = Flat::new();
        flatten("", &table, &mut flat)?;

        let table: toml::Table =
            content.parse().map_err(|e| format!("content/locales/{code}.toml: {e}"))?;
        let mut personal = Flat::new();
        flatten("", &table, &mut personal)?;
        for (key, value) in personal {
            if flat.insert(key.clone(), value).is_some() {
                return Err(format!(
                    "`{key}` is defined in both locales/{code}.toml and content/locales/{code}.toml"
                ));
            }
        }
        map.insert(code, flat);
    }

    let reference = &map[Lang::En.code()];
    for lang in Lang::ALL {
        let flat = &map[lang.code()];
        let mut missing: Vec<_> = reference.keys().filter(|k| !flat.contains_key(*k)).collect();
        let mut extra: Vec<_> = flat.keys().filter(|k| !reference.contains_key(*k)).collect();
        missing.sort();
        extra.sort();
        if !missing.is_empty() || !extra.is_empty() {
            return Err(format!(
                "locales/{}.toml (+ content/locales) is out of sync with en.toml\n  missing: {missing:?}\n  extra: {extra:?}",
                lang.code()
            ));
        }
    }
    // Keys required by structured content (projects, skills) must exist in every language.
    for key in crate::content::required_locale_keys() {
        for lang in Lang::ALL {
            if !map[lang.code()].contains_key(&key) {
                return Err(format!(
                    "locales/{}.toml is missing `{key}` (required by content/*.toml)",
                    lang.code()
                ));
            }
        }
    }
    Ok(Store(map))
}

/// Per-request translator. Also usable as a Rocket request guard.
#[derive(Clone, Copy)]
pub struct I18n {
    pub lang: Lang,
}

impl I18n {
    pub fn t(&self, key: &str) -> String {
        let found = STORE.get().and_then(|s| s.read().unwrap().0[self.lang.code()].get(key).cloned());
        match found {
            // Debug builds: tag the text with its key for the in-page editor (editor.rs).
            #[cfg(debug_assertions)]
            Some(s) => crate::editor::mark(key, &s),
            #[cfg(not(debug_assertions))]
            Some(s) => s,
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
