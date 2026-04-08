use std::path::Path;

fn main() {
    // Tell cargo to rerun if the web UI dist changes
    println!("cargo:rerun-if-changed=../../web-ui/dist");
    println!("cargo:rerun-if-changed=../../web-ui/src");

    // Verify web-ui/dist exists — fail fast with helpful message if not
    let dist = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../web-ui/dist");
    if !dist.exists() {
        panic!(
            "web-ui/dist not found — run `cd web-ui && npm run build` first\n\
             Expected path: {}",
            dist.display()
        );
    }

    // Verify index.html exists in dist
    let index = dist.join("index.html");
    if !index.exists() {
        panic!(
            "web-ui/dist/index.html missing — the Svelte build may have failed\n\
             Run `cd web-ui && npm run build` to regenerate"
        );
    }
}
