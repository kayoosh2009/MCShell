#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::Command;

#[tauri::command]
fn launch_game(nickname: String, version: String, game_dir: String, java_path: String) -> Result<String, String> {
    if nickname.trim().is_empty() {
        return Err("Введите никнейм!".into());
    }

    // Путь к JAR-файлу версии (например, .minecraft/versions/1.20.1/1.20.1.jar)
    let version_jar = format!("{}/versions/{}/{}.jar", game_dir, version, version);

    // В полноценном лаунчере здесь собирается список всех .jar из папки libraries.
    // Для базового PoC передаем основной клиентский JAR:
    let classpath = version_jar;

    // Формируем аргументы запуска Minecraft
    let args = vec![
        "-Xmx2G".to_string(), // Выделяем 2 ГБ ОЗУ
        format!("-Djava.library.path={}/versions/{}/natives", game_dir, version),
        "-cp".to_string(),
        classpath,
        "net.minecraft.client.main.Main".to_string(), // Главный класс игры
        "--username".to_string(), nickname,
        "--version".to_string(), version,
        "--gameDir".to_string(), game_dir,
        "--assetsDir".to_string(), format!("{}/assets", game_dir),
        "--assetIndex".to_string(), version,
        "--accessToken".to_string(), "0".to_string(), // Токен для офлайн-режима
        "--userProperties".to_string(), "{}".to_string(),
    ];

    // Запускаем процесс Minecraft
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