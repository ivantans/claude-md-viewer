// Claude MD Viewer: the application shell, shared by macOS, Windows and Linux.
// Each window hosts the system webview showing Resources/viewer.html. The shell reads the Markdown
// file, passes its text to the page's render() function, and renders again when the file changes
// on disk. Files are only ever read, never written.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    thread,
    time::{Duration, SystemTime},
};

use serde_json::json;
use tauri::{
    http::{header::CONTENT_TYPE, Request, Response, StatusCode},
    menu::{AboutMetadata, CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    webview::{Color, NewWindowResponse},
    AppHandle, DragDropEvent, Manager, Theme, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Window,
    WindowEvent, Wry,
};
use tauri_plugin_dialog::DialogExt;

const APP_NAME: &str = "Claude MD Viewer";
const ABOUT: &str = "An unofficial, community-made Markdown viewer.\nNot affiliated with or endorsed by Anthropic.";
const MARKDOWN_EXTENSIONS: [&str; 5] = ["md", "markdown", "mdown", "mkd", "txt"];
// Images referenced by a document are served by the "mdfile" protocol below. The page's <base>
// points at it, so relative paths resolve against the Markdown file's folder.
const IMAGE_TYPES: [(&str, &str); 10] = [
    ("png", "image/png"),
    ("apng", "image/apng"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("webp", "image/webp"),
    ("avif", "image/avif"),
    ("svg", "image/svg+xml"),
    ("bmp", "image/bmp"),
    ("ico", "image/x-icon"),
];
// Local link targets that would run something instead of showing it are not opened.
const PROGRAM_EXTENSIONS: [&str; 24] = [
    "app", "command", "desktop", "sh", "jar", "exe", "bat", "cmd", "com", "scr", "pif", "msi", "msp", "ps1", "vbs",
    "vbe", "js", "jse", "wsf", "wsh", "hta", "lnk", "url", "reg",
];
#[cfg(windows)]
const FILE_ORIGIN: &str = "http://mdfile.localhost";
#[cfg(not(windows))]
const FILE_ORIGIN: &str = "mdfile://localhost";

const DARK_BACKGROUND: Color = Color(0x1c, 0x1b, 0x19, 0xff);
const LIGHT_BACKGROUND: Color = Color(0xfa, 0xf9, 0xf5, 0xff);

struct Document {
    path: Option<PathBuf>,
    modified: Option<SystemTime>,
    page_loaded: bool,
    zoom: f64,
}

// Dark by default, like the Markdown preview in the Claude app.
struct Settings {
    theme: String,
    width: f64,
    height: f64,
}

#[derive(Default)]
struct Viewer {
    documents: Mutex<HashMap<String, Document>>,
    focused: Mutex<Option<String>>,
    settings: Mutex<Option<Settings>>,
    theme_items: Mutex<Vec<(&'static str, CheckMenuItem<Wry>)>>,
    // Held while a file is being opened so two requests cannot create two windows for one file.
    // Never taken on the main thread, which has to stay free to create the window.
    opening: Mutex<()>,
    next_window: AtomicUsize,
}

fn extension(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase()
}

fn is_markdown(path: &Path) -> bool {
    MARKDOWN_EXTENSIONS.contains(&extension(path).as_str())
}

fn is_program(path: &Path) -> bool {
    if PROGRAM_EXTENSIONS.contains(&extension(path).as_str()) {
        return true;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0) {
            return true;
        }
    }
    false
}

fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

// MARK: Settings

fn settings_file(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|dir| dir.join("settings.json"))
}

fn with_settings<T>(app: &AppHandle, f: impl FnOnce(&mut Settings) -> T) -> T {
    let state = app.state::<Viewer>();
    let mut settings = state.settings.lock().unwrap();
    let settings = settings.get_or_insert_with(|| {
        let saved: serde_json::Value = settings_file(app)
            .and_then(|file| fs::read(file).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Settings {
            theme: match saved["theme"].as_str() {
                Some(theme @ ("light" | "system")) => theme.into(),
                _ => "dark".into(),
            },
            width: saved["width"].as_f64().filter(|w| (480.0..=10000.0).contains(w)).unwrap_or(1040.0),
            height: saved["height"].as_f64().filter(|h| (360.0..=10000.0).contains(h)).unwrap_or(900.0),
        }
    });
    f(settings)
}

fn save_settings(app: &AppHandle) {
    let Some(file) = settings_file(app) else { return };
    let saved = with_settings(app, |s| json!({ "theme": s.theme, "width": s.width, "height": s.height }));
    if let Some(dir) = file.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let _ = fs::write(file, saved.to_string());
}

// MARK: Theme

fn chosen_theme(app: &AppHandle) -> Option<Theme> {
    with_settings(app, |s| match s.theme.as_str() {
        "light" => Some(Theme::Light),
        "system" => None,
        _ => Some(Theme::Dark),
    })
}

fn background(theme: Theme) -> Color {
    if theme == Theme::Light { LIGHT_BACKGROUND } else { DARK_BACKGROUND }
}

// The page follows prefers-color-scheme, which follows the theme set on the app.
fn apply_theme(app: &AppHandle) {
    let theme = chosen_theme(app);
    app.set_theme(theme);
    let name = with_settings(app, |s| s.theme.clone());
    let items = app.state::<Viewer>().theme_items.lock().unwrap().clone();
    for (id, item) in items {
        let _ = item.set_checked(id.strip_prefix("theme-") == Some(name.as_str()));
    }
    for window in app.webview_windows().values() {
        if let Some(theme) = theme.or_else(|| window.theme().ok()) {
            let _ = window.set_background_color(Some(background(theme)));
        }
    }
}

// MARK: Documents

fn active_window(app: &AppHandle) -> Option<WebviewWindow> {
    let focused = app.state::<Viewer>().focused.lock().unwrap().clone();
    focused
        .and_then(|label| app.get_webview_window(&label))
        .or_else(|| app.webview_windows().into_values().next())
}

fn focus(window: &WebviewWindow) {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

fn base_url(folder: &Path) -> String {
    Url::from_directory_path(folder).map(|url| format!("{FILE_ORIGIN}{}", url.path())).unwrap_or_default()
}

/// Reads the window's file and hands it to the page.
fn render(app: &AppHandle, label: &str, keep_scroll: bool) {
    let Some(window) = app.get_webview_window(label) else { return };
    let state = app.state::<Viewer>();
    let path = match state.documents.lock().unwrap().get(label) {
        Some(Document { path: Some(path), page_loaded: true, .. }) => path.clone(),
        _ => return,
    };
    let Ok(bytes) = fs::read(&path) else { return };
    if let Some(document) = state.documents.lock().unwrap().get_mut(label) {
        document.modified = modified(&path);
    }
    let text = String::from_utf8_lossy(&bytes);
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let folder = path.parent().unwrap_or(Path::new(""));
    let payload = json!({
        "markdown": text.trim_start_matches('\u{feff}'),
        "name": name,
        "folder": folder.file_name().unwrap_or_default().to_string_lossy(),
        "baseURL": base_url(folder),
        "keepScroll": keep_scroll,
    });
    let _ = window.set_title(&name);
    let _ = window.eval(format!("render({payload})"));
}

/// Shows `path` in an existing window, replacing whatever it showed before.
fn show(app: &AppHandle, label: &str, path: PathBuf) {
    if let Some(document) = app.state::<Viewer>().documents.lock().unwrap().get_mut(label) {
        document.path = Some(path);
        document.modified = None;
    }
    render(app, label, false);
}

/// Opens `path` in its own window, or just makes sure a window exists when there is no path.
/// Blocks until the window exists, so it must not run on the main thread: see `open_later`.
fn open(app: &AppHandle, path: Option<PathBuf>) {
    let state = app.state::<Viewer>();
    let _opening = state.opening.lock().unwrap();
    let path = path.map(|path| std::path::absolute(&path).unwrap_or(path));

    let existing = {
        let documents = state.documents.lock().unwrap();
        let find = |wanted: Option<&PathBuf>| {
            documents.iter().find(|(_, d)| d.path.as_ref() == wanted).map(|(label, _)| label.clone())
        };
        match &path {
            // A file that is already open is brought to the front; otherwise an empty window is reused.
            Some(path) => find(Some(path)).map(|label| (label, false)).or_else(|| find(None).map(|label| (label, true))),
            None => documents.keys().next().map(|label| (label.clone(), false)),
        }
    };
    if let Some((label, empty)) = existing {
        if let (true, Some(path)) = (empty, path) {
            show(app, &label, path);
        }
        if let Some(window) = app.get_webview_window(&label) {
            focus(&window);
        }
        return;
    }

    let label = format!("doc-{}", state.next_window.fetch_add(1, Ordering::Relaxed));
    let title = path.as_ref().and_then(|p| p.file_name()).map_or(APP_NAME.into(), |name| name.to_string_lossy());
    let (width, height) = with_settings(app, |s| (s.width, s.height));
    let mut builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("viewer.html".into()))
        .title(title)
        .inner_size(width, height)
        .min_inner_size(480.0, 360.0)
        // Links never replace the viewer page; the page hands them to open_link instead.
        .on_navigation(|url| {
            url.scheme() == "tauri"
                || url.host_str() == Some("tauri.localhost")
                || (tauri::is_dev() && matches!(url.host_str(), Some("localhost" | "127.0.0.1")))
        })
        .on_new_window(|_, _| NewWindowResponse::Deny);
    if let Some(theme) = chosen_theme(app) {
        builder = builder.background_color(background(theme)); // avoids a white flash before the page loads
    }
    #[cfg(target_os = "macos")]
    {
        builder = builder.title_bar_style(tauri::TitleBarStyle::Transparent);
    }
    // Cascade from the frontmost window.
    let front = active_window(app).and_then(|w| Some(w.outer_position().ok()?.to_logical::<f64>(w.scale_factor().ok()?)));
    builder = match front {
        Some(position) => builder.position(position.x + 26.0, position.y + 26.0),
        None => builder.center(),
    };

    let document = Document { path, modified: None, page_loaded: false, zoom: 1.0 };
    state.documents.lock().unwrap().insert(label.clone(), document);
    match builder.build() {
        Ok(window) => {
            if chosen_theme(app).is_none() {
                let _ = window.set_background_color(window.theme().ok().map(background));
            }
        }
        Err(_) => {
            state.documents.lock().unwrap().remove(&label);
        }
    }
}

/// Opens files from the main thread. Creating a window there deadlocks on Windows, and `open`
/// waits for the main thread, so the work moves to its own thread.
fn open_later(app: &AppHandle, paths: Vec<PathBuf>) {
    let app = app.clone();
    thread::spawn(move || {
        if paths.is_empty() {
            open(&app, None);
        }
        for path in paths {
            open(&app, Some(path));
        }
    });
}

/// File paths among command line arguments, as passed by "Open with" on Windows and Linux.
fn files_in(arguments: impl Iterator<Item = String>, folder: &Path) -> Vec<PathBuf> {
    arguments.skip(1).filter(|a| !a.starts_with('-')).map(|a| folder.join(a)).filter(|p| p.is_file()).collect()
}

fn choose_files(app: &AppHandle) {
    let handle = app.clone();
    let mut dialog = app.dialog().file().add_filter("Markdown", &MARKDOWN_EXTENSIONS);
    if let Some(window) = active_window(app) {
        dialog = dialog.set_parent(&window);
    }
    dialog.pick_files(move |files| {
        let paths: Vec<PathBuf> = files.into_iter().flatten().filter_map(|file| file.into_path().ok()).collect();
        if !paths.is_empty() {
            open_later(&handle, paths);
        }
    });
}

/// Re-renders every window whose file changed on disk, keeping its scroll position.
fn watch_files(app: AppHandle) {
    loop {
        thread::sleep(Duration::from_secs(1));
        let changed: Vec<String> = {
            let state = app.state::<Viewer>();
            let documents = state.documents.lock().unwrap();
            let is_changed = |d: &Document| {
                d.page_loaded && d.path.as_deref().and_then(modified).is_some_and(|current| Some(current) != d.modified)
            };
            documents.iter().filter(|(_, d)| is_changed(d)).map(|(label, _)| label.clone()).collect()
        };
        for label in changed {
            render(&app, &label, true);
        }
    }
}

// MARK: Commands called by the page (Resources/viewer.js)

/// The page finished loading: render the window's file, if it has one.
#[tauri::command]
fn ready(app: AppHandle, window: WebviewWindow) {
    if let Some(document) = app.state::<Viewer>().documents.lock().unwrap().get_mut(window.label()) {
        document.page_loaded = true;
    }
    render(&app, window.label(), false);
}

/// The path of a link that points into the file system, relative links included.
fn local_path(url: &Url) -> Option<PathBuf> {
    let local = matches!(url.scheme(), "file" | "mdfile") || url.host_str() == Some("mdfile.localhost");
    if !local {
        return None;
    }
    Url::parse(&format!("file://{}", url.path())).ok()?.to_file_path().ok()
}

/// A link was clicked: other Markdown files open in a new window, everything else is handed to
/// the default browser or app.
#[tauri::command]
async fn open_link(app: AppHandle, url: String) {
    let Ok(url) = Url::parse(&url) else { return };
    match local_path(&url) {
        Some(path) if is_markdown(&path) => open(&app, Some(path)),
        Some(path) => {
            if path.exists() && !is_program(&path) {
                let _ = open::that_detached(path);
            }
        }
        None => {
            if matches!(url.scheme(), "http" | "https" | "mailto") {
                let _ = open::that_detached(url.as_str());
            }
        }
    }
}

/// A keyboard shortcut the page caught before the menu could (see the end of viewer.js).
#[tauri::command]
async fn action(app: AppHandle, window: WebviewWindow, id: String) {
    run_action(&app, Some(window), &id);
}

// MARK: Menu

fn run_action(app: &AppHandle, window: Option<WebviewWindow>, id: &str) {
    match id {
        "open" => return choose_files(app),
        "close" => return drop(window.or_else(|| active_window(app)).map(|window| window.close())),
        "quit" => return app.exit(0),
        "theme-dark" | "theme-light" | "theme-system" => {
            with_settings(app, |s| s.theme = id["theme-".len()..].into());
            save_settings(app);
            return apply_theme(app);
        }
        _ => {}
    }
    let Some(window) = window.or_else(|| active_window(app)) else { return };
    let zoom = |change: fn(f64) -> f64| {
        let state = app.state::<Viewer>();
        let zoom = state.documents.lock().unwrap().get_mut(window.label()).map(|document| {
            document.zoom = change(document.zoom);
            document.zoom
        });
        let _ = window.set_zoom(zoom.unwrap_or(1.0));
    };
    match id {
        "reload" => render(app, window.label(), true),
        // Tauri's print() is only implemented on macOS, where the page's own window.print() does nothing.
        #[cfg(target_os = "macos")]
        "print" => drop(window.print()),
        #[cfg(not(target_os = "macos"))]
        "print" => drop(window.eval("window.print()")),
        "copy" => drop(window.eval("document.execCommand('copy')")),
        "select-all" => drop(window.eval("document.execCommand('selectAll')")),
        "find" => drop(window.eval("openFind()")),
        "find-next" => drop(window.eval("findNext(false)")),
        "toc" => drop(window.eval("toggleTOC()")),
        "zoom-in" => zoom(|z| (z + 0.1).min(2.5)),
        "zoom-out" => zoom(|z| (z - 0.1).max(0.6)),
        "zoom-reset" => zoom(|_| 1.0),
        _ => {}
    }
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let item = |id: &str, text: &str, key: &str| {
        MenuItem::with_id(app, id, text, true, (!key.is_empty()).then_some(key))
    };
    let separator = || PredefinedMenuItem::separator(app);
    let about = PredefinedMenuItem::about(
        app,
        Some(&format!("About {APP_NAME}")),
        Some(AboutMetadata {
            name: Some(APP_NAME.into()),
            version: Some(app.package_info().version.to_string()),
            // macOS shows the credits, Windows and Linux the comments.
            credits: cfg!(target_os = "macos").then(|| ABOUT.into()),
            comments: (!cfg!(target_os = "macos")).then(|| ABOUT.into()),
            // macOS already shows the application icon.
            icon: app.default_window_icon().filter(|_| !cfg!(target_os = "macos")).cloned(),
            ..Default::default()
        }),
    )?;

    let file = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &item("open", "Open…", "CmdOrCtrl+O")?,
            &item("reload", "Reload", "CmdOrCtrl+R")?,
            &separator()?,
            &item("print", "Print…", "CmdOrCtrl+P")?,
        ],
    )?;
    // On macOS the predefined item closes whichever window has the keyboard, the About panel included.
    if cfg!(target_os = "macos") {
        file.append(&PredefinedMenuItem::close_window(app, Some("Close Window"))?)?;
    } else {
        file.append(&item("close", "Close Window", "CmdOrCtrl+W")?)?;
    }
    let edit = Submenu::new(app, "Edit", true)?;
    // The predefined items do nothing on Linux unless libxdo is installed, so the page is asked
    // instead. Ctrl+C and Ctrl+A are handled by the webview itself there.
    if cfg!(target_os = "linux") {
        edit.append_items(&[&item("copy", "Copy", "")?, &item("select-all", "Select All", "")?])?;
    } else {
        edit.append_items(&[&PredefinedMenuItem::copy(app, None)?, &PredefinedMenuItem::select_all(app, None)?])?;
    }
    edit.append_items(&[
        &separator()?,
        &item("find", "Find…", "CmdOrCtrl+F")?,
        &item("find-next", "Find Next", "CmdOrCtrl+G")?,
    ])?;

    let chosen = with_settings(app, |s| s.theme.clone());
    let theme = |id: &'static str, text: &str| {
        let item = CheckMenuItem::with_id(app, id, text, true, id.ends_with(chosen.as_str()), None::<&str>)?;
        app.state::<Viewer>().theme_items.lock().unwrap().push((id, item.clone()));
        tauri::Result::Ok(item)
    };
    let view = Submenu::with_items(
        app,
        "View",
        true,
        &[
            &item("toc", "Table of Contents", "CmdOrCtrl+Shift+O")?,
            &separator()?,
            &theme("theme-dark", "Dark")?,
            &theme("theme-light", "Light")?,
            &theme("theme-system", "Match System")?,
            &separator()?,
            &item("zoom-in", "Zoom In", "CmdOrCtrl+=")?,
            &item("zoom-out", "Zoom Out", "CmdOrCtrl+-")?,
            &item("zoom-reset", "Actual Size", "CmdOrCtrl+0")?,
        ],
    )?;

    #[cfg(target_os = "macos")]
    {
        let app_menu = Submenu::with_items(
            app,
            APP_NAME,
            true,
            &[
                &about,
                &separator()?,
                &PredefinedMenuItem::hide(app, None)?,
                &PredefinedMenuItem::hide_others(app, None)?,
                &PredefinedMenuItem::show_all(app, None)?,
                &separator()?,
                &PredefinedMenuItem::quit(app, None)?,
            ],
        )?;
        let window = Submenu::with_id_and_items(
            app,
            tauri::menu::WINDOW_SUBMENU_ID,
            "Window",
            true,
            &[&PredefinedMenuItem::minimize(app, None)?, &PredefinedMenuItem::maximize(app, Some("Zoom"))?],
        )?;
        Menu::with_items(app, &[&app_menu, &file, &edit, &view, &window])
    }
    #[cfg(not(target_os = "macos"))]
    {
        file.append_items(&[&separator()?, &item("quit", "Quit", "CmdOrCtrl+Q")?])?;
        let help = Submenu::with_items(app, "Help", true, &[&about])?;
        Menu::with_items(app, &[&file, &edit, &view, &help])
    }
}

// MARK: Windows

fn on_window_event(window: &Window, event: &WindowEvent) {
    let app = window.app_handle();
    let state = app.state::<Viewer>();
    match event {
        WindowEvent::Focused(true) => *state.focused.lock().unwrap() = Some(window.label().into()),
        // A Markdown file dropped onto the window replaces the document it shows.
        WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) => {
            if let Some(path) = paths.iter().find(|path| is_markdown(path)) {
                show(app, window.label(), path.clone());
            }
        }
        // With "Match System" the window background follows the system theme.
        WindowEvent::ThemeChanged(theme) if chosen_theme(app).is_none() => {
            if let Some(window) = app.get_webview_window(window.label()) {
                let _ = window.set_background_color(Some(background(*theme)));
            }
        }
        // New windows open at the size of the last one that was closed.
        WindowEvent::CloseRequested { .. } => {
            let filling_screen = window.is_maximized().unwrap_or(false) || window.is_fullscreen().unwrap_or(false);
            if let (false, Ok(size), Ok(scale)) = (filling_screen, window.inner_size(), window.scale_factor()) {
                let size = size.to_logical::<f64>(scale);
                with_settings(app, |s| (s.width, s.height) = (size.width, size.height));
                save_settings(app);
            }
        }
        WindowEvent::Destroyed => {
            state.documents.lock().unwrap().remove(window.label());
        }
        _ => {}
    }
}

/// Serves images from disk to the page. Anything that is not an image is refused.
fn serve_image(request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let respond = |status: StatusCode, content_type: &str, body: Vec<u8>| {
        Response::builder()
            .status(status)
            .header(CONTENT_TYPE, content_type)
            .header("X-Content-Type-Options", "nosniff")
            .body(body)
            .unwrap()
    };
    let path = Url::parse(&format!("file://{}", request.uri().path())).ok().and_then(|url| url.to_file_path().ok());
    let Some(path) = path else { return respond(StatusCode::BAD_REQUEST, "text/plain", Vec::new()) };
    let extension = extension(&path);
    let Some((_, content_type)) = IMAGE_TYPES.iter().find(|(known, _)| *known == extension) else {
        return respond(StatusCode::FORBIDDEN, "text/plain", Vec::new());
    };
    match fs::read(&path) {
        Ok(bytes) => respond(StatusCode::OK, content_type, bytes),
        Err(_) => respond(StatusCode::NOT_FOUND, "text/plain", Vec::new()),
    }
}

fn main() {
    let current_folder = || std::env::current_dir().unwrap_or_default();
    tauri::Builder::default()
        // A second launch (for example "Open with" on Windows and Linux) hands its files to the
        // running instance and exits.
        .plugin(tauri_plugin_single_instance::init(|app, arguments, folder| {
            open_later(app, files_in(arguments.into_iter(), Path::new(&folder)));
        }))
        .plugin(tauri_plugin_dialog::init())
        .manage(Viewer::default())
        .register_asynchronous_uri_scheme_protocol("mdfile", |_context, request, responder| {
            thread::spawn(move || responder.respond(serve_image(request)));
        })
        .invoke_handler(tauri::generate_handler![ready, open_link, action])
        .menu(build_menu)
        .on_menu_event(|app, event| run_action(app, None, event.id().as_ref()))
        .on_window_event(on_window_event)
        .setup(move |app| {
            let app = app.handle();
            apply_theme(app);
            // On macOS a file opened from Finder arrives as RunEvent::Opened below; whichever
            // comes first, the empty window created here is reused for it.
            open_later(app, files_in(std::env::args(), &current_folder()));
            let watcher = app.clone();
            thread::spawn(move || watch_files(watcher));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to start Claude MD Viewer")
        .run(|_app, _event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Opened { urls } = _event {
                open_later(_app, urls.iter().filter_map(|url| url.to_file_path().ok()).collect());
            }
        });
}
