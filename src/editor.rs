//! Dev-only in-page text editor. In debug builds (`cargo run`) every translated string
//! is wrapped in invisible markers that name its key (see [`mark`]). `editor.js` turns
//! those into hover targets: hover a text, click the ✎ button (or Alt+click the text)
//! and edit the English and Dutch versions right there, with a live preview on the page.
//! Saving writes into the locale TOML file that defines the key (comments and layout
//! are kept) and reloads the running site's strings.
//!
//! Release builds don't include any of this. It only edits values of existing keys,
//! so the languages can't drift out of sync.

use std::sync::Mutex;

use maud::{Markup, html};
use rocket::Route;
use rocket::data::{Data, ToByteUnit};
use rocket::http::ContentType;
use rocket::response::status::BadRequest;
use toml_edit::{DocumentMut, Item, Value};

use crate::i18n::{self, Lang, LocaleFile};

pub fn routes() -> Vec<Route> {
    routes![script, style, value, save]
}

/// `<link>`/`<script>` tags for the page `<head>`.
pub fn head() -> Markup {
    html! {
        link rel="stylesheet" href="/edit/editor.css";
        script src="/edit/editor.js" defer {}
    }
}

// Invisible marker characters. A marked string looks like
// START <key as bits: ZERO/ONE per bit, 8 per byte> SEP <text> END
const START: char = '\u{2063}';
const SEP: char = '\u{2064}';
const END: char = '\u{2062}';
const ZERO: char = '\u{200B}';
const ONE: char = '\u{200C}';

/// Wrap a translated string in invisible markers naming its key.
pub fn mark(key: &str, text: &str) -> String {
    let mut out = String::with_capacity(text.len() + key.len() * 24 + 12);
    out.push(START);
    for byte in key.bytes() {
        for bit in (0..8).rev() {
            out.push(if byte >> bit & 1 == 1 { ONE } else { ZERO });
        }
    }
    out.push(SEP);
    out.push_str(text);
    out.push(END);
    out
}

/// Serializes saves so two quick edits can't clobber each other's write.
static WRITE_LOCK: Mutex<()> = Mutex::new(());

fn read_doc(file: LocaleFile, lang: Lang) -> Result<DocumentMut, String> {
    let path = i18n::locale_path(file, lang);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    text.parse().map_err(|e| format!("{path}: {e}"))
}

fn lookup<'a>(doc: &'a DocumentMut, key: &str) -> Option<&'a str> {
    let mut item: &Item = doc.as_item();
    for part in key.split('.') {
        item = item.as_table_like()?.get(part)?;
    }
    item.as_str()
}

/// Which locale file defines `key` (personal copy first, then UI strings).
fn file_for(key: &str) -> Result<LocaleFile, String> {
    for file in [LocaleFile::Content, LocaleFile::Ui] {
        if lookup(&read_doc(file, Lang::En)?, key).is_some() {
            return Ok(file);
        }
    }
    Err(format!("unknown key `{key}`"))
}

/// Replace the string at `key` (dotted path), keeping its surrounding whitespace/comments.
fn set_value(doc: &mut DocumentMut, key: &str, value: &str) -> Result<(), String> {
    let mut item: &mut Item = doc.as_item_mut();
    for part in key.split('.') {
        item = item
            .as_table_like_mut()
            .and_then(|t| t.get_mut(part))
            .ok_or_else(|| format!("unknown key `{key}`"))?;
    }
    let old = item.as_value().filter(|v| v.is_str()).ok_or_else(|| format!("`{key}` is not a string"))?;
    let mut new = Value::from(value);
    *new.decor_mut() = old.decor().clone();
    *item = Item::Value(new);
    Ok(())
}

fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Raw (unfilled) values of a key in every language, as JSON.
#[get("/edit/value?<key>")]
fn value(key: &str) -> Result<(ContentType, String), BadRequest<String>> {
    let file = file_for(key).map_err(BadRequest)?;
    let mut fields = vec![format!("\"file\":{}", json_str(&i18n::locale_path(file, Lang::En)))];
    for lang in Lang::ALL {
        let doc = read_doc(file, lang).map_err(BadRequest)?;
        let v = lookup(&doc, key).unwrap_or_default();
        fields.push(format!("{}:{}", json_str(lang.code()), json_str(v)));
    }
    Ok((ContentType::JSON, format!("{{{}}}", fields.join(","))))
}

/// Save one string. Query: `key=dotted.path`, `lang=en|nl`; body: the new text.
#[post("/edit/save?<key>&<lang>", data = "<body>")]
async fn save(key: &str, lang: &str, body: Data<'_>) -> Result<&'static str, BadRequest<String>> {
    let bad = |e: String| BadRequest(e);
    let lang = Lang::parse(lang).ok_or_else(|| bad(format!("unknown language `{lang}`")))?;
    let value = body.open(256.kibibytes()).into_string().await.map_err(|e| bad(e.to_string()))?;
    if !value.is_complete() {
        return Err(bad("text too long".into()));
    }
    let value = value.into_inner().replace('\r', "");

    let _guard = WRITE_LOCK.lock().unwrap();
    let file = file_for(key).map_err(bad)?;
    let path = i18n::locale_path(file, lang);
    let mut doc = read_doc(file, lang).map_err(bad)?;
    if lookup(&doc, key) == Some(value.as_str()) {
        return Ok("unchanged");
    }
    set_value(&mut doc, key, &value).map_err(bad)?;
    std::fs::write(&path, doc.to_string()).map_err(|e| bad(format!("{path}: {e}")))?;
    i18n::reload_from_disk().map_err(|e| bad(format!("saved, but reload failed: {e}")))?;
    Ok("saved")
}

#[get("/edit/editor.js")]
fn script() -> (ContentType, &'static str) {
    (ContentType::JavaScript, JS)
}

#[get("/edit/editor.css")]
fn style() -> (ContentType, &'static str) {
    (ContentType::CSS, CSS)
}

const CSS: &str = r#"
[data-i18n] { transition: outline-color .1s; outline: 1px dashed transparent; outline-offset: 2px; }
.ed-on [data-i18n].ed-hover { outline-color: var(--accent); }
.ed-on [data-i18n].ed-editing { outline: 2px solid var(--accent); }
.ed-pencil { position: fixed; z-index: 1000; display: none; width: 1.6rem; height: 1.6rem; padding: 0;
  font: 14px/1 var(--font-body); cursor: pointer; color: var(--on-accent); background: var(--accent);
  border: 0; box-shadow: 2px 2px 0 var(--bg); }
.ed-pop { position: fixed; z-index: 1001; width: min(36rem, calc(100vw - 2rem)); display: grid; gap: .5rem;
  padding: .75rem; background: var(--bg-raised); border: 2px solid var(--accent); box-shadow: var(--shadow-dim);
  font: 14px/1.4 var(--font-body); color: var(--fg); }
.ed-pop header { display: flex; justify-content: space-between; gap: .5rem; color: var(--fg-dim); font-size: 12px; }
.ed-pop header code { color: var(--fg); overflow-wrap: anywhere; }
.ed-pop label { display: grid; gap: .2rem; font-size: 12px; color: var(--fg-dim); }
.ed-pop label.cur span::after { content: " · this page"; color: var(--accent); }
.ed-pop textarea { font: inherit; font-size: 14px; color: var(--fg); background: var(--bg); resize: vertical;
  border: 2px solid var(--border); padding: .3rem .5rem; min-height: 2.2rem; max-height: 50vh; box-sizing: border-box; width: 100%; }
.ed-pop textarea:focus { outline: none; border-color: var(--fg-dim); }
.ed-pop footer { display: flex; gap: .5rem; align-items: center; }
.ed-pop footer .ed-msg { margin-right: auto; font-size: 12px; color: var(--fg-dim); }
.ed-pop footer .ed-msg.err { color: var(--accent); }
.ed-pop button, .ed-dock button { font: inherit; font-size: 13px; cursor: pointer; padding: .3rem .7rem;
  color: var(--fg); background: var(--bg); border: 2px solid var(--border); }
.ed-pop button.primary { color: var(--on-accent); background: var(--accent); border-color: var(--accent); }
.ed-dock { position: fixed; z-index: 999; right: 1rem; bottom: 1rem; display: grid; justify-items: end; gap: .4rem;
  font: 13px/1.4 var(--font-body); }
.ed-dock ul { margin: 0; padding: .4rem; list-style: none; background: var(--bg-raised); border: 2px solid var(--border);
  max-height: 40vh; overflow: auto; }
.ed-dock li button { border: 0; background: none; text-align: left; width: 100%; padding: .2rem .4rem; }
.ed-dock li button:hover { color: var(--accent); }
.ed-dock .on { color: var(--on-accent); background: var(--accent); border-color: var(--accent); }
"#;

const JS: &str = r#"
(() => {
  const RE = /⁣([​‌]+)⁤([\s\S]*?)⁢/g;
  const STRIP = /⁣[​‌]*⁤|⁢/g;
  const pageLang = document.documentElement.lang || 'en';
  const LANGS = ['en', 'nl'];
  const decodeKey = bits => {
    const bytes = new Uint8Array(bits.length / 8);
    for (let i = 0; i < bits.length; i++) if (bits[i] === '‌') bytes[i >> 3] |= 128 >> (i & 7);
    return new TextDecoder().decode(bytes);
  };

  // 1. Turn marked text into <span data-i18n>; collect keys only used in attributes / <title>.
  const hidden = new Set();
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  const nodes = [];
  while (walker.nextNode()) if (walker.currentNode.data.includes('⁣')) nodes.push(walker.currentNode);
  for (const node of nodes) {
    const frag = document.createDocumentFragment();
    let last = 0;
    for (const m of node.data.matchAll(RE)) {
      if (m.index > last) frag.append(node.data.slice(last, m.index));
      const span = document.createElement('span');
      span.dataset.i18n = decodeKey(m[1]);
      span.textContent = m[2];
      frag.append(span);
      last = m.index + m[0].length;
    }
    frag.append(node.data.slice(last).replace(STRIP, ''));
    node.replaceWith(frag);
  }
  for (const el of document.querySelectorAll('*')) {
    for (const a of el.attributes) {
      if (!a.value.includes('⁣')) continue;
      for (const m of a.value.matchAll(RE)) hidden.add(decodeKey(m[1]));
      a.value = a.value.replace(STRIP, '');
    }
  }
  for (const m of document.title.matchAll(RE)) hidden.add(decodeKey(m[1]));
  document.title = document.title.replace(STRIP, '');
  const spansOf = key => [...document.querySelectorAll('[data-i18n]')].filter(s => s.dataset.i18n === key);
  for (const s of document.querySelectorAll('[data-i18n]')) hidden.delete(s.dataset.i18n);

  // 2. Hover pencil.
  let on = true;
  try { on = localStorage.getItem('ed-off') !== '1'; } catch {}
  document.documentElement.classList.toggle('ed-on', on);
  const pencil = Object.assign(document.createElement('button'), { className: 'ed-pencil', textContent: '✎', title: 'Edit text (or Alt+click)' });
  document.body.append(pencil);
  let target = null, hideTimer = 0;
  const showPencil = el => {
    clearTimeout(hideTimer);
    if (target && target !== el) target.classList.remove('ed-hover');
    target = el;
    el.classList.add('ed-hover');
    const r = el.getBoundingClientRect();
    pencil.style.left = Math.min(r.right + 4, innerWidth - 30) + 'px';
    pencil.style.top = Math.max(r.top - 14, 4) + 'px';
    pencil.style.display = 'block';
  };
  const hidePencil = () => {
    hideTimer = setTimeout(() => {
      pencil.style.display = 'none';
      target && target.classList.remove('ed-hover');
      target = null;
    }, 400);
  };
  document.addEventListener('mouseover', e => {
    if (!on || pop) return;
    const el = e.target.closest && e.target.closest('[data-i18n]');
    if (el) showPencil(el);
    else if (e.target === pencil) clearTimeout(hideTimer);
    else if (target) hidePencil();
  });
  addEventListener('scroll', () => { if (target && !pop) showPencil(target); }, { passive: true });
  pencil.addEventListener('click', e => { e.preventDefault(); if (target) open(target.dataset.i18n, target); });
  document.addEventListener('click', e => {
    const el = on && e.altKey && e.target.closest('[data-i18n]');
    if (el) { e.preventDefault(); e.stopPropagation(); open(el.dataset.i18n, el); }
  }, true);

  // 3. Edit popover.
  let pop = null;
  const esc = s => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  // Placeholder values ({year}, {name}) recovered by matching the template against the rendered text.
  const argsFor = (template, rendered) => {
    const names = [...template.matchAll(/\{(\w+)\}/g)].map(m => m[1]);
    if (!names.length) return {};
    const re = new RegExp('^' + esc(template).replace(/\\\{(\w+)\\\}/g, '([\\s\\S]*?)') + '$');
    const m = rendered.match(re);
    return m ? Object.fromEntries(names.map((n, i) => [n, m[i + 1]])) : {};
  };
  const fill = (template, args) => template.replace(/\{(\w+)\}/g, (all, n) => n in args ? args[n] : all);

  async function open(key, anchor) {
    close();
    pencil.style.display = 'none';
    const r = await fetch('/edit/value?key=' + encodeURIComponent(key));
    if (!r.ok) return alert(await r.text());
    const data = await r.json();
    const spans = spansOf(key);
    const original = spans.map(s => s.textContent);
    const args = spans.length ? argsFor(data[pageLang], original[0]) : {};
    spans.forEach(s => s.classList.add('ed-editing'));

    pop = document.createElement('div');
    pop.className = 'ed-pop';
    pop.innerHTML = `<header><code></code><span class="ed-file"></span></header>` +
      LANGS.map(l => `<label class="${l === pageLang ? 'cur' : ''}"><span>${l.toUpperCase()}</span><textarea data-lang="${l}" spellcheck="true" lang="${l}"></textarea></label>`).join('') +
      `<footer><span class="ed-msg">Ctrl+Enter save · Esc cancel</span><button type="button" data-act="cancel">Cancel</button><button type="button" class="primary" data-act="save">Save</button></footer>`;
    pop.querySelector('code').textContent = key;
    pop.querySelector('.ed-file').textContent = data.file.replace(/^.*\/((content\/|examples\/content\/)?locales\/)/, '$1').replace(/\w+\.toml$/, '*.toml');
    const areas = [...pop.querySelectorAll('textarea')];
    const fit = t => { t.style.height = 'auto'; t.style.height = t.scrollHeight + 4 + 'px'; };
    for (const t of areas) {
      t.value = data[t.dataset.lang];
      t.addEventListener('input', () => {
        fit(t);
        if (t.dataset.lang === pageLang) spans.forEach(s => s.textContent = fill(t.value, args));
        place();
      });
    }
    document.body.append(pop);
    areas.forEach(fit);
    const place = () => {
      const a = (anchor.isConnected ? anchor : spans[0] || pencil).getBoundingClientRect();
      const h = pop.offsetHeight, w = pop.offsetWidth;
      const top = a.bottom + 8 + h < innerHeight ? a.bottom + 8 : Math.max(8, a.top - 8 - h);
      pop.style.top = top + 'px';
      pop.style.left = Math.max(8, Math.min(a.left, innerWidth - w - 8)) + 'px';
    };
    place();
    const cur = areas.find(t => t.dataset.lang === pageLang) || areas[0];
    cur.focus();
    cur.setSelectionRange(cur.value.length, cur.value.length);

    const msg = pop.querySelector('.ed-msg');
    const cancel = () => { spans.forEach((s, i) => s.textContent = original[i]); close(); };
    const save = async () => {
      msg.textContent = 'saving…'; msg.className = 'ed-msg';
      try {
        for (const t of areas) {
          if (t.value === data[t.dataset.lang]) continue;
          const q = new URLSearchParams({ key, lang: t.dataset.lang });
          const res = await fetch('/edit/save?' + q, { method: 'POST', body: t.value });
          if (!res.ok) throw new Error(await res.text());
          data[t.dataset.lang] = t.value;
        }
        close();
        if (!spans.length) location.reload(); // e.g. the page title: re-render to show it
      } catch (e) {
        msg.textContent = e.message; msg.className = 'ed-msg err';
      }
    };
    pop.addEventListener('click', e => {
      if (e.target.dataset.act === 'save') save();
      if (e.target.dataset.act === 'cancel') cancel();
    });
    pop.addEventListener('keydown', e => {
      if (e.key === 'Escape') cancel();
      if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) { e.preventDefault(); save(); }
    });
    pop._cancel = cancel;
    pop._place = place;
  }
  function close() {
    if (!pop) return;
    pop.remove();
    pop = null;
    document.querySelectorAll('.ed-editing').forEach(s => s.classList.remove('ed-editing'));
  }
  addEventListener('scroll', () => pop && pop._place(), { passive: true });
  addEventListener('resize', () => pop && pop._place());
  document.addEventListener('mousedown', e => {
    if (pop && !pop.contains(e.target) && e.target !== pencil) pop._cancel();
  });

  // 4. Dock: edit-mode toggle + texts that aren't visible on the page (title, tooltips).
  const dock = document.createElement('div');
  dock.className = 'ed-dock';
  const list = document.createElement('ul');
  list.hidden = true;
  for (const key of [...hidden].sort()) {
    const li = document.createElement('li');
    const b = Object.assign(document.createElement('button'), { textContent: key, type: 'button' });
    b.addEventListener('click', () => { list.hidden = true; open(key, toggle); });
    li.append(b);
    list.append(li);
  }
  const row = document.createElement('div');
  const more = Object.assign(document.createElement('button'), { textContent: `hidden texts (${hidden.size})`, type: 'button', title: 'Page title and other texts not shown on the page' });
  more.addEventListener('click', () => { list.hidden = !list.hidden; });
  const toggle = Object.assign(document.createElement('button'), { type: 'button' });
  const render = () => { toggle.textContent = on ? '✎ editing' : '✎ off'; toggle.classList.toggle('on', on); more.hidden = !on || !hidden.size; if (!on) list.hidden = true; };
  toggle.addEventListener('click', () => {
    on = !on;
    try { localStorage.setItem('ed-off', on ? '0' : '1'); } catch {}
    document.documentElement.classList.toggle('ed-on', on);
    if (!on) { close(); pencil.style.display = 'none'; }
    render();
  });
  render();
  row.append(more, ' ', toggle);
  dock.append(list, row);
  document.body.append(dock);
})();
"#;
