use serde::{Deserialize, Serialize};
use std::fs;
use tauri::{AppHandle, Manager, Runtime};
use tauri::http::{Response, StatusCode};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub main: String,
    #[serde(rename = "tagName")]
    pub tag_name: String,
    pub description: Option<String>,
}

#[tauri::command]
pub fn get_plugin_list<R: Runtime>(app: AppHandle<R>) -> Result<Vec<PluginManifest>, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let plugins_dir = app_data_dir.join("plugins");
    
    println!("Scanning plugins directory: {:?}", plugins_dir);

    if !plugins_dir.exists() {
        println!("Plugins directory does not exist, creating...");
        fs::create_dir_all(&plugins_dir).map_err(|e| e.to_string())?;
        return Ok(vec![]);
    }

    let mut plugins = Vec::new();
    let entries = fs::read_dir(&plugins_dir).map_err(|e| e.to_string())?;

    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_dir() {
                let manifest_path = path.join("manifest.json");
                if manifest_path.exists() {
                    match fs::read_to_string(&manifest_path) {
                        Ok(content) => {
                            match serde_json::from_str::<PluginManifest>(&content) {
                                Ok(manifest) => plugins.push(manifest),
                                Err(e) => println!("Failed to parse manifest for {:?}: {}", path, e),
                            }
                        },
                        Err(e) => println!("Failed to read manifest for {:?}: {}", path, e),
                    }
                }
            }
        }
    }

    Ok(plugins)
}

#[tauri::command]
pub fn load_plugin_file<R: Runtime>(app: AppHandle<R>, plugin_id: String, file_name: String) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let plugin_dir = app_data_dir.join("plugins").join(&plugin_id);
    let target_path = plugin_dir.join(&file_name);
    
    // Security check
    if file_name.contains("..") || !target_path.starts_with(&plugin_dir) {
        return Err("Access denied".to_string());
    }

    fs::read_to_string(target_path).map_err(|e| e.to_string())
}

pub fn plugin_protocol_handler<R: Runtime>(
    context: tauri::UriSchemeContext<'_, R>,
    request: tauri::http::Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let app = context.app_handle();
    match handle_request(app, request) {
        Ok(response) => response,
        Err(e) => {
            println!("Plugin protocol error: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(e.to_string().into_bytes())
                .unwrap_or_default()
        }
    }
}

fn handle_request<R: Runtime>(
    app: &AppHandle<R>,
    request: tauri::http::Request<Vec<u8>>,
) -> Result<Response<Vec<u8>>, Box<dyn std::error::Error>> {
    let uri = request.uri();
    let path_str = uri.path();
    
    // uri path is like "/<plugin-id>/<file-path>"
    // e.g., "plugin://demo-widget/index.js" -> path is "/demo-widget/index.js"
    
    // Remove leading slash
    let path_str = path_str.trim_start_matches('/');
    
    // Split into plugin_id and file_path
    let parts: Vec<&str> = path_str.splitn(2, '/').collect();
    
    if parts.len() < 2 {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(vec![])
            .map_err(|e| e.into());
    }
    
    let plugin_id = parts[0];
    let file_path = parts[1];
    
    // Security check: prevent directory traversal
    if file_path.contains("..") {
         return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(vec![])
            .map_err(|e| e.into());
    }

    let app_data_dir = app.path().app_data_dir()?;
    let plugin_dir = app_data_dir.join("plugins").join(plugin_id);
    let target_path = plugin_dir.join(file_path);
    
    // Ensure the target path is actually within the plugin directory (double check)
    if !target_path.starts_with(&plugin_dir) {
         return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(vec![])
            .map_err(|e| e.into());
    }

    match fs::read(&target_path) {
        Ok(content) => {
            let mime_type = mime_guess::from_path(&target_path).first_or_octet_stream();
            Response::builder()
                .header("Access-Control-Allow-Origin", "*")
                .header("Content-Type", mime_type.as_ref())
                .body(content)
                .map_err(|e| e.into())
        },
        Err(_) => {
             Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(vec![])
                .map_err(|e| e.into())
        }
    }
}
