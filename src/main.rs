mod content;
mod i18n;
mod pages;
mod site;
mod theme;
mod ui;

use rocket::fs::FileServer;
use rocket::http::CookieJar;
use rocket::response::Redirect;

use i18n::Lang;

#[macro_use]
extern crate rocket;

/// Switch language: stores a cookie, then sends the visitor back to where they were.
#[get("/lang/<code>")]
fn set_lang(code: &str, jar: &CookieJar<'_>, referer: Referer) -> Redirect {
    if let Some(lang) = Lang::parse(code) {
        i18n::set_lang_cookie(jar, lang);
    }
    Redirect::to(referer.0)
}

/// Same-site path of the Referer header (falls back to "/").
struct Referer(String);

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for Referer {
    type Error = std::convert::Infallible;
    async fn from_request(req: &'r rocket::Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let path = req
            .headers()
            .get_one("Referer")
            .and_then(|r| r.split_once("://").map(|(_, rest)| rest))
            .and_then(|rest| rest.find('/').map(|i| &rest[i..]))
            .filter(|p| p.starts_with('/') && !p.starts_with("//"))
            .unwrap_or("/");
        rocket::request::Outcome::Success(Referer(path.to_string()))
    }
}

/// `static/` is resolved relative to the working directory (the repo root in dev,
/// the install dir under systemd), not baked in at compile time.
#[launch]
fn launch_app() -> _ {
    site::load();
    content::load();
    i18n::load();
    rocket::build()
        .mount("/", pages::routes())
        .mount("/", routes![set_lang])
        .mount("/static", FileServer::from("static"))
        .register("/", catchers![pages::not_found])
}
