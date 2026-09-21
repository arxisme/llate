mod db;
mod lsp;

use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use tauri_plugin_shell::ShellExt;
use rusqlite::Connection;
use tauri::{State, Manager};

struct DbState(Mutex<Option<Connection>>);
struct LspState(Mutex<Option<u16>>);

#[derive(Serialize, Deserialize)]
pub struct NoteSummary {
    pub path: String,
    pub title: String,
}

#[derive(Serialize)]
pub struct CompileResult {
    pub pdf_bytes: Option<Vec<u8>>,
    pub synctex_bytes: Option<Vec<u8>>,
    pub log: String,
    pub ok: bool,
}

#[tauri::command]
fn get_lsp_port(lsp_state: State<LspState>) -> Result<u16, String> {
    if let Some(port) = *lsp_state.0.lock().unwrap() {
        Ok(port)
    } else {
        Err("LSP not initialized".to_string())
    }
}

#[tauri::command]
fn list_notes(vault_path: &str, db_state: State<DbState>) -> Result<Vec<NoteSummary>, String> {
    let mut notes = Vec::new();
    let entries = fs::read_dir(vault_path).map_err(|e| e.to_string())?;
    
    // Initialize DB on first access
    let mut db_guard = db_state.0.lock().unwrap();
    if db_guard.is_none() {
        let conn = db::init_db(Path::new(vault_path)).map_err(|e| e.to_string())?;
        *db_guard = Some(conn);
    }
    
    let conn = db_guard.as_ref().unwrap();

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        
        let ext = path.extension().and_then(|s| s.to_str());
        if path.is_file() && (ext == Some("tex") || ext == Some("bib") || ext == Some("txt") || ext == Some("md")) {
            let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let title = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
            notes.push(NoteSummary {
                path: file_name.clone(),
                title: title.clone(),
            });
            
            // Re-index note on startup
            if let Ok(content) = fs::read_to_string(&path) {
                let _ = db::update_note_index(conn, &file_name, &title, &content);
            }
        }
    }
    Ok(notes)
}

#[tauri::command]
fn read_note(vault_path: &str, note_path: &str) -> Result<String, String> {
    let path = Path::new(vault_path).join(note_path);
    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_note(vault_path: &str, note_path: &str, content: &str, db_state: State<DbState>) -> Result<(), String> {
    let path = Path::new(vault_path).join(note_path);
    fs::write(&path, content).map_err(|e| e.to_string())?;
    
    let title = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    if let Some(conn) = db_state.0.lock().unwrap().as_ref() {
        db::update_note_index(conn, note_path, &title, content).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn compile_note(_app: tauri::AppHandle, vault_path: String, note_path: String, content: String) -> Result<CompileResult, String> {
    let path = std::path::Path::new(&vault_path).join(&note_path);
    std::fs::write(&path, content).map_err(|e| e.to_string())?;

    let path_str = path.to_str().unwrap();
    let current_exe = std::env::current_exe().unwrap();
    let dir = current_exe.parent().unwrap();
    
    let tectonic_path = if dir.join("tectonic-x86_64-pc-windows-msvc.exe").exists() {
        dir.join("tectonic-x86_64-pc-windows-msvc.exe")
    } else if dir.join("bin/tectonic-x86_64-pc-windows-msvc.exe").exists() {
        dir.join("bin/tectonic-x86_64-pc-windows-msvc.exe")
    } else if dir.join("../bin/tectonic-x86_64-pc-windows-msvc.exe").exists() {
        dir.join("../bin/tectonic-x86_64-pc-windows-msvc.exe")
    } else {
        std::path::PathBuf::from("tectonic")
    };

    let mut cmd = tokio::process::Command::new(&tectonic_path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    
    let output = cmd
        .args(["-X", "compile", "--synctex", &path_str])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let log = String::from_utf8_lossy(&output.stderr).to_string();
    
    Ok(CompileResult {
        ok: output.status.success(),
        log,
        pdf_bytes: None,
        synctex_bytes: None,
    })
}

#[tauri::command]
fn open_sumatra(pdf_path: String, tex_path: String, line: i32) -> Result<(), String> {
    use std::process::Command;
    let sumatra_path = r#"C:\Users\varia\AppData\Local\SumatraPDF\SumatraPDF.exe"#;
    
    let mut cmd = Command::new(sumatra_path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    
    cmd.arg("-reuse-instance")
        .arg("-forward-search")
        .arg(&tex_path)
        .arg(line.to_string())
        .arg(&pdf_path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_backlinks(note_path: &str, db_state: State<DbState>) -> Result<Vec<String>, String> {
    if let Some(conn) = db_state.0.lock().unwrap().as_ref() {
        db::get_backlinks(conn, note_path).map_err(|e| e.to_string())
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
fn get_tags(note_path: &str, db_state: State<DbState>) -> Result<Vec<String>, String> {
    if let Some(conn) = db_state.0.lock().unwrap().as_ref() {
        db::get_tags_for_note(conn, note_path).map_err(|e| e.to_string())
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
fn get_all_tags(db_state: State<DbState>) -> Result<Vec<String>, String> {
    if let Some(conn) = db_state.0.lock().unwrap().as_ref() {
        db::get_all_tags(conn).map_err(|e| e.to_string())
    } else {
        Ok(Vec::new())
    }
}
#[tauri::command]
fn search_notes(query: &str, db_state: State<DbState>) -> Result<Vec<String>, String> {
    if let Some(conn) = db_state.0.lock().unwrap().as_ref() {
        db::search_notes(conn, query).map_err(|e| e.to_string())
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
fn get_graph_data(db_state: State<DbState>) -> Result<db::GraphData, String> {
    if let Some(conn) = db_state.0.lock().unwrap().as_ref() {
        db::get_graph_data(conn).map_err(|e| e.to_string())
    } else {
        Ok(db::GraphData { nodes: vec![], links: vec![] })
    }
}

#[tauri::command]
fn create_file(vault_path: &str, file_name: &str) -> Result<(), String> {
    let path = Path::new(vault_path).join(file_name);
    if !path.exists() {
        fs::write(&path, "").map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn create_folder(vault_path: &str, folder_name: &str) -> Result<(), String> {
    let path = Path::new(vault_path).join(folder_name);
    if !path.exists() {
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn rename_file(vault_path: &str, old_name: &str, new_name: &str, db_state: State<DbState>) -> Result<(), String> {
    let old_path = Path::new(vault_path).join(old_name);
    let new_path = Path::new(vault_path).join(new_name);
    
    if old_path.exists() {
        fs::rename(&old_path, &new_path).map_err(|e| e.to_string())?;
    }
    
    // Rename associated files
    let extensions = ["pdf", "synctex.gz", "log", "aux"];
    let old_stem = Path::new(old_name).file_stem().unwrap_or_default().to_string_lossy();
    let new_stem = Path::new(new_name).file_stem().unwrap_or_default().to_string_lossy();
    
    for ext in extensions {
        let old_assoc = Path::new(vault_path).join(format!("{}.{}", old_stem, ext));
        let new_assoc = Path::new(vault_path).join(format!("{}.{}", new_stem, ext));
        if old_assoc.exists() {
            let _ = fs::rename(old_assoc, new_assoc);
        }
    }

    if let Some(conn) = db_state.0.lock().unwrap().as_ref() {
        let _ = db::delete_note_index(conn, old_name);
        if let Ok(content) = fs::read_to_string(&new_path) {
            let _ = db::update_note_index(conn, new_name, &new_stem, &content);
        }
    }

    Ok(())
}

#[tauri::command]
fn delete_file(vault_path: &str, file_name: &str, db_state: State<DbState>) -> Result<(), String> {
    let path = Path::new(vault_path).join(file_name);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    
    let extensions = ["pdf", "synctex.gz", "log", "aux"];
    let stem = Path::new(file_name).file_stem().unwrap_or_default().to_string_lossy();
    
    for ext in extensions {
        let assoc = Path::new(vault_path).join(format!("{}.{}", stem, ext));
        if assoc.exists() {
            let _ = fs::remove_file(assoc);
        }
    }

    if let Some(conn) = db_state.0.lock().unwrap().as_ref() {
        let _ = db::delete_note_index(conn, file_name);
    }
    
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(port) = lsp::start_lsp_server().await {
                    app_handle.manage(LspState(Mutex::new(Some(port))));
                } else {
                    eprintln!("Failed to start LSP server");
                }
            });
            Ok(())
        })
        .manage(DbState(Mutex::new(None)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            list_notes, read_note, save_note, compile_note, open_sumatra, create_file, create_folder,
            rename_file, delete_file,
            get_backlinks, get_tags, get_all_tags, search_notes, get_graph_data, get_lsp_port
        ])

        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
