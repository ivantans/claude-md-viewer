fn main() {
    // Declaring the commands makes them subject to the capability in capabilities/default.json.
    let manifest = tauri_build::AppManifest::new().commands(&["ready", "open_link", "action"]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest)).expect("failed to run tauri-build");
}
