//! Picks the content directory that gets embedded into the binary:
//! `content/` (private, the real site) if present, else `examples/content/`
//! (placeholder data shipped with the public repo).

use std::path::Path;

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = if root.join("content").is_dir() {
        root.join("content")
    } else {
        println!("cargo:warning=content/ not found, building with examples/content/");
        root.join("examples/content")
    };
    println!("cargo:rustc-env=CONTENT_DIR={}", dir.display());
    // Re-run when content/ appears or disappears; the files themselves are tracked by include_str!.
    println!("cargo:rerun-if-changed=content");
    println!("cargo:rerun-if-changed=build.rs");
}
