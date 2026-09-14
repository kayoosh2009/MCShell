#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::Command;

#[tauri::command]
fn launch_game(nickname: String, version: String, game_dir: String, java_path: String) -> Result<String, String> {
    if nickname.trim().is_empty() {
        return Err("Введите никнейм!".into());
    }

    // Раскрываем символ ~ в домашнюю директорию, если пользователь передал путь с ним
    let expanded_dir = if game_dir.starts_with("~") {
        let home = std::env::var("HOME").unwrap_or_default();
        game_dir.replacen("~", &home, 1)
    } else {
        game_dir
    };

    let version_jar = format!("{}/versions/{}/{}.jar", expanded_dir, version, version);
    let classpath = version_jar;

    let args = vec![
        "-Xmx2G".to_string(),
        format!("-Djava.library.path={}/versions/{}/natives", expanded_dir, version),
        "-cp".to_string(),
        classpath,
        "net.minecraft.client.main.Main".to_string(),
        "--username".to_string(), nickname,
        "--version".to_string(), version,
        "--gameDir".to_string(), expanded_dir.clone(),
        "--assetsDir".to_string(), format!("{}/assets", expanded_dir),
        "--assetIndex".to_string(), version,
        "--accessToken".to_string(), "0".to_string(),
        "--userProperties".to_string(), "{}".to_string(),
    ];

    match Command::new(java_path).args(&args).spawn() {
        Ok(_) => Ok("Игра запускается!".into()),
        Err(e) => Err(format!("Ошибка запуска Java: {}", e)),
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![launch_game])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}