# portfolio

My personal portfolio site: a single Rust binary built with
[Rocket](https://rocket.rs) and [maud](https://maud.lambda.xyz), with
English/Dutch translations and a terminal-inspired theme.

> This repository is a read-only mirror. The personal content and deployment
> config live in a private repo and are stripped out before publishing.

## Running it

```sh
cargo run
# → http://127.0.0.1:8000
```

Without a `content/` directory, the build falls back to the placeholder data in
[`examples/content/`](examples/content), so the site works out of the box.

## Layout

| Path                 | What                                                         |
| -------------------- | ------------------------------------------------------------ |
| `src/`               | Routes, pages, i18n, theme                                   |
| `locales/`           | UI strings per language (`en.toml`, `nl.toml`)               |
| `static/`            | CSS and fonts, served from disk at runtime                   |
| `examples/content/`  | Example personal data (projects, skills, bio, links)         |
| `build.rs`           | Embeds `content/` if present, else `examples/content/`       |

To make it your own, copy `examples/content/` to `content/` and edit it. At
startup the app checks that every language defines the same keys, and that every
project and skill category has its translations. It refuses to start if any
are missing.

## License

The code is licensed under the [MIT License](LICENSE).

The bundled fonts in `static/fonts/` are **not** covered by the MIT license. Each
one is licensed under the SIL Open Font License 1.1:

- IBM Plex Mono © IBM Corp.: [`OFL-IBMPlexMono.txt`](static/fonts/OFL-IBMPlexMono.txt)
- Departure Mono © Helena Zhang: [`OFL-DepartureMono.txt`](static/fonts/OFL-DepartureMono.txt)
