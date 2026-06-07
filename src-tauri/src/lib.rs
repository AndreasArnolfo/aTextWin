use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub id: String,
    pub abbreviation: String,
    pub expansion: String,
    pub enabled: bool,
    #[serde(default)]
    pub group: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Stats {
    pub total_expansions: u64,
    pub chars_saved: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// When true, an abbreviation only triggers if preceded by a non-alphanumeric
    /// character (or the start of the buffer) — prevents expansion mid-word.
    #[serde(default = "default_true")]
    pub require_word_boundary: bool,
    /// Process executable names (e.g. "keepass.exe") where expansion is disabled.
    #[serde(default)]
    pub blacklist: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            require_word_boundary: true,
            blacklist: Vec::new(),
        }
    }
}

pub struct AppState {
    pub snippets: Vec<Snippet>,
    pub buffer: String,
    pub active: bool,
    pub is_expanding: bool,
    pub data_path: PathBuf,
    pub stats: Stats,
    pub stats_path: PathBuf,
    pub settings: Settings,
    pub settings_path: PathBuf,
}

impl AppState {
    fn new(data_path: PathBuf, stats_path: PathBuf, settings_path: PathBuf) -> Self {
        let snippets = load_snippets(&data_path).unwrap_or_default();
        let stats = load_stats(&stats_path).unwrap_or_default();
        let settings = load_settings(&settings_path).unwrap_or_default();
        Self {
            snippets,
            buffer: String::new(),
            active: true,
            is_expanding: false,
            data_path,
            stats,
            stats_path,
            settings,
            settings_path,
        }
    }
}

fn load_snippets(path: &PathBuf) -> Option<Vec<Snippet>> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_snippets(state: &AppState) {
    if let Some(parent) = state.data_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(content) = serde_json::to_string_pretty(&state.snippets) {
        let _ = std::fs::write(&state.data_path, content);
    }
}

fn load_stats(path: &PathBuf) -> Option<Stats> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_stats(state: &AppState) {
    if let Some(parent) = state.stats_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(content) = serde_json::to_string_pretty(&state.stats) {
        let _ = std::fs::write(&state.stats_path, content);
    }
}

fn load_settings(path: &PathBuf) -> Option<Settings> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_settings(state: &AppState) {
    if let Some(parent) = state.settings_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(content) = serde_json::to_string_pretty(&state.settings) {
        let _ = std::fs::write(&state.settings_path, content);
    }
}

fn unique_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros()
        .to_string()
}

// --- Tauri commands ---

#[tauri::command]
fn get_snippets(state: tauri::State<'_, Arc<Mutex<AppState>>>) -> Vec<Snippet> {
    state.lock().unwrap().snippets.clone()
}

#[tauri::command]
fn add_snippet(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    abbreviation: String,
    expansion: String,
    group: String,
) -> Snippet {
    let snippet = Snippet {
        id: unique_id(),
        abbreviation,
        expansion,
        enabled: true,
        group,
    };
    let mut s = state.lock().unwrap();
    s.snippets.push(snippet.clone());
    save_snippets(&s);
    snippet
}

#[tauri::command]
fn update_snippet(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    id: String,
    abbreviation: String,
    expansion: String,
    enabled: bool,
    group: String,
) -> bool {
    let mut s = state.lock().unwrap();
    if let Some(sn) = s.snippets.iter_mut().find(|sn| sn.id == id) {
        sn.abbreviation = abbreviation;
        sn.expansion = expansion;
        sn.enabled = enabled;
        sn.group = group;
        save_snippets(&s);
        true
    } else {
        false
    }
}

#[tauri::command]
fn delete_snippet(state: tauri::State<'_, Arc<Mutex<AppState>>>, id: String) -> bool {
    let mut s = state.lock().unwrap();
    let before = s.snippets.len();
    s.snippets.retain(|sn| sn.id != id);
    if s.snippets.len() < before {
        save_snippets(&s);
        true
    } else {
        false
    }
}

#[tauri::command]
fn export_snippets(state: tauri::State<'_, Arc<Mutex<AppState>>>, path: String) -> Result<(), String> {
    let s = state.lock().unwrap();
    let content = serde_json::to_string_pretty(&s.snippets).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_snippets(state: tauri::State<'_, Arc<Mutex<AppState>>>, path: String) -> Result<usize, String> {
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let imported: Vec<Snippet> = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    let mut s = state.lock().unwrap();
    let count = imported.len();
    for (i, mut sn) in imported.into_iter().enumerate() {
        sn.id = format!("{}-{}", unique_id(), i);
        s.snippets.push(sn);
    }
    save_snippets(&s);
    Ok(count)
}

#[tauri::command]
fn get_active(state: tauri::State<'_, Arc<Mutex<AppState>>>) -> bool {
    state.lock().unwrap().active
}

#[tauri::command]
fn set_active(state: tauri::State<'_, Arc<Mutex<AppState>>>, active: bool) {
    let mut s = state.lock().unwrap();
    s.active = active;
    s.buffer.clear();
}

#[tauri::command]
fn get_stats(state: tauri::State<'_, Arc<Mutex<AppState>>>) -> Stats {
    state.lock().unwrap().stats.clone()
}

#[tauri::command]
fn reset_stats(state: tauri::State<'_, Arc<Mutex<AppState>>>) {
    let mut s = state.lock().unwrap();
    s.stats = Stats::default();
    save_stats(&s);
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, Arc<Mutex<AppState>>>) -> Settings {
    state.lock().unwrap().settings.clone()
}

#[tauri::command]
fn update_settings(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    require_word_boundary: bool,
    blacklist: Vec<String>,
) {
    let mut s = state.lock().unwrap();
    s.settings.require_word_boundary = require_word_boundary;
    s.settings.blacklist = blacklist
        .into_iter()
        .map(|b| b.trim().to_lowercase())
        .filter(|b| !b.is_empty())
        .collect();
    save_settings(&s);
}

/// Returns the lowercased executable file name of the process owning the
/// foreground window (e.g. "notepad.exe"), used to apply the app blacklist.
fn foreground_process_name() -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::ProcessStatus::K32GetModuleBaseNameW;
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0 == 0 {
            return None;
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }

        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ, false, pid).ok()?;

        let mut buffer = [0u16; 260];
        let len = K32GetModuleBaseNameW(process, None, &mut buffer);
        let _ = CloseHandle(process);

        if len == 0 {
            return None;
        }

        Some(String::from_utf16_lossy(&buffer[..len as usize]).to_lowercase())
    }
}

// --- Keyboard hook ---

fn run_keyboard_hook(
    state: Arc<Mutex<AppState>>,
    tx: std::sync::mpsc::SyncSender<(usize, String)>,
) {
    use rdev::{listen, EventType, Key};

    if let Err(e) = listen(move |event| {
        // Fast early exit: don't process while expanding or inactive
        {
            let s = state.lock().unwrap();
            if s.is_expanding || !s.active {
                return;
            }
        }

        if let EventType::KeyPress(key) = event.event_type {
            match key {
                // Word boundary keys → clear buffer
                Key::Space
                | Key::Return
                | Key::Escape
                | Key::Tab
                | Key::CapsLock => {
                    state.lock().unwrap().buffer.clear();
                }
                // Editing keys → adjust or clear buffer
                Key::Backspace => {
                    state.lock().unwrap().buffer.pop();
                }
                Key::LeftArrow
                | Key::RightArrow
                | Key::UpArrow
                | Key::DownArrow
                | Key::Home
                | Key::End
                | Key::Delete => {
                    state.lock().unwrap().buffer.clear();
                }
                _ => {
                    // Only handle printable single characters
                    if let Some(name) = &event.name {
                        let chars: Vec<char> = name.chars().collect();
                        if chars.len() == 1 {
                            let c = chars[0];
                            if !c.is_control() {
                                let maybe_match = {
                                    let mut s = state.lock().unwrap();
                                    s.buffer.push(c);

                                    // Keep buffer bounded (max 128 chars)
                                    if s.buffer.len() > 128 {
                                        let excess = s.buffer.len() - 128;
                                        s.buffer.drain(..excess);
                                    }

                                    let buffer_chars: Vec<char> = s.buffer.chars().collect();
                                    let require_boundary = s.settings.require_word_boundary;

                                    // Match longest abbreviation first, honoring the word-boundary setting
                                    let mut snippets: Vec<(usize, String)> = s
                                        .snippets
                                        .iter()
                                        .filter(|sn| sn.enabled && !sn.abbreviation.is_empty())
                                        .filter_map(|sn| {
                                            let abbr_chars: Vec<char> = sn.abbreviation.chars().collect();
                                            if abbr_chars.len() > buffer_chars.len() {
                                                return None;
                                            }
                                            let start = buffer_chars.len() - abbr_chars.len();
                                            if buffer_chars[start..] != abbr_chars[..] {
                                                return None;
                                            }
                                            if require_boundary && start > 0 {
                                                let prev = buffer_chars[start - 1];
                                                if prev.is_alphanumeric() {
                                                    return None;
                                                }
                                            }
                                            Some((abbr_chars.len(), sn.expansion.clone()))
                                        })
                                        .collect();

                                    snippets.sort_by(|a, b| b.0.cmp(&a.0));
                                    snippets.into_iter().next()
                                };

                                if let Some((abbr_len, expansion)) = maybe_match {
                                    // Respect the application blacklist
                                    let blacklisted = {
                                        let s = state.lock().unwrap();
                                        if s.settings.blacklist.is_empty() {
                                            false
                                        } else {
                                            foreground_process_name()
                                                .map(|name| s.settings.blacklist.iter().any(|b| b == &name))
                                                .unwrap_or(false)
                                        }
                                    };

                                    state.lock().unwrap().buffer.clear();

                                    if !blacklisted {
                                        // Non-blocking send: if the worker is busy, skip
                                        let _ = tx.try_send((abbr_len, expansion));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }) {
        eprintln!("rdev listen error: {:?}", e);
    }
}

/// Replaces dynamic variables in an expansion and extracts the cursor placement.
/// Returns the processed text and, if `{curseur}` was present, the number of
/// characters to move the cursor left after typing (i.e. characters that followed
/// the marker).
fn process_variables(text: &str) -> (String, Option<usize>) {
    let mut result = text.to_string();

    let now = chrono::Local::now();
    result = result.replace("{date}", &now.format("%d/%m/%Y").to_string());
    result = result.replace("{heure}", &now.format("%H:%M").to_string());
    result = result.replace("{datetime}", &now.format("%d/%m/%Y %H:%M").to_string());

    if result.contains("{clipboard}") {
        let clip = arboard::Clipboard::new()
            .and_then(|mut c| c.get_text())
            .unwrap_or_default();
        result = result.replace("{clipboard}", &clip);
    }

    let cursor_offset = result.find("{curseur}").map(|pos| {
        let chars_after = result[pos + "{curseur}".len()..].chars().count();
        result = result.replacen("{curseur}", "", 1);
        chars_after
    });

    (result, cursor_offset)
}

fn run_expansion_worker(
    state: Arc<Mutex<AppState>>,
    rx: std::sync::mpsc::Receiver<(usize, String)>,
) {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    while let Ok((abbr_len, expansion)) = rx.recv() {
        state.lock().unwrap().is_expanding = true;

        // Small delay so the triggering keystroke is fully processed first
        std::thread::sleep(std::time::Duration::from_millis(30));

        if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
            // Delete the abbreviation character by character
            for _ in 0..abbr_len {
                let _ = enigo.key(Key::Backspace, Direction::Click);
            }

            std::thread::sleep(std::time::Duration::from_millis(10));

            // Resolve variables, then type the expansion text
            let (text, cursor_offset) = process_variables(&expansion);
            let _ = enigo.text(&text);

            // Track usage statistics (characters saved = typed text minus the abbreviation)
            {
                let mut s = state.lock().unwrap();
                s.stats.total_expansions += 1;
                let diff = text.chars().count() as i64 - abbr_len as i64;
                if diff > 0 {
                    s.stats.chars_saved += diff;
                }
                save_stats(&s);
            }

            // Move the cursor back to the {curseur} marker position
            if let Some(offset) = cursor_offset {
                std::thread::sleep(std::time::Duration::from_millis(10));
                for _ in 0..offset {
                    let _ = enigo.key(Key::LeftArrow, Direction::Click);
                }
            }
        }

        state.lock().unwrap().is_expanding = false;
    }
}

// --- Entry point ---

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized".into()]),
        ))
        .setup(|app| {
            // Don't pop up the window when launched at Windows startup
            let start_minimized = std::env::args().any(|a| a == "--minimized");

            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data dir");
            let data_path = app_data_dir.join("snippets.json");
            let stats_path = app_data_dir.join("stats.json");
            let settings_path = app_data_dir.join("settings.json");

            let state = Arc::new(Mutex::new(AppState::new(data_path, stats_path, settings_path)));

            // Bounded channel: capacity 1 prevents stacking expansions
            let (tx, rx) = std::sync::mpsc::sync_channel::<(usize, String)>(1);

            let hook_state = Arc::clone(&state);
            let hook_tx = tx.clone();
            std::thread::spawn(move || {
                run_keyboard_hook(hook_state, hook_tx);
            });

            let expand_state = Arc::clone(&state);
            std::thread::spawn(move || {
                run_expansion_worker(expand_state, rx);
            });

            // Clone before manage() moves state
            let state_for_tray = Arc::clone(&state);
            app.manage(state);

            // Hide to tray on close instead of quitting
            let window = app.get_webview_window("main").unwrap();

            if start_minimized {
                let _ = window.hide();
            }

            let win_for_close = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = win_for_close.hide();
                }
            });

            // Global hotkey to show/hide the main window from anywhere
            {
                use tauri_plugin_global_shortcut::GlobalShortcutExt;
                let shortcut: tauri_plugin_global_shortcut::Shortcut =
                    "CommandOrControl+Shift+Space".parse().expect("invalid shortcut");
                if let Err(e) = app.global_shortcut().register(shortcut) {
                    eprintln!("Failed to register global shortcut: {:?}", e);
                }
            }

            // System tray
            let initial_active = state_for_tray.lock().unwrap().active;

            let show_item = MenuItem::with_id(app, "show", "Afficher aTextWin", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let active_item = CheckMenuItem::with_id(app, "active", "Actif", true, initial_active, None::<&str>)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quitter aTextWin", true, None::<&str>)?;

            let tray_menu = Menu::with_items(app, &[&show_item, &sep1, &active_item, &sep2, &quit_item])?;

            let active_for_tray = active_item.clone();

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .tooltip("aTextWin — Text Expansion")
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                    "active" => {
                        let mut s = state_for_tray.lock().unwrap();
                        s.active = !s.active;
                        let new_val = s.active;
                        s.buffer.clear();
                        drop(s);
                        let _ = active_for_tray.set_checked(new_val);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snippets,
            add_snippet,
            update_snippet,
            delete_snippet,
            get_active,
            set_active,
            export_snippets,
            import_snippets,
            get_stats,
            reset_stats,
            get_settings,
            update_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
