use std::path::Path;

fn main() {
    emit_app_version();
    tauri_build::build();
}

fn emit_app_version() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let config_path = Path::new(&manifest_dir).join("../../../config.yaml");
    println!("cargo:rerun-if-changed={}", config_path.display());

    let cargo_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();

    let version = std::fs::read_to_string(&config_path)
        .ok()
        .and_then(|content| parse_version(&content))
        .unwrap_or_else(|| cargo_version.clone());

    if !cargo_version.is_empty() && version != cargo_version {
        println!(
            "cargo:warning=config.yaml version ({version}) differs from Cargo.toml ({cargo_version}); \
             the app shows config.yaml. Update Cargo.toml, tauri.conf.json and package.json to match before releasing."
        );
    }

    println!("cargo:rustc-env=AUDIS_VERSION={version}");
}

fn parse_version(yaml: &str) -> Option<String> {
    for line in yaml.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("version:") {
            let value = rest.trim().trim_matches('"').trim_matches('\'').trim();
            if !value.is_empty() {
                return Some(value.to_owned());
            }
        }
    }
    None
}
