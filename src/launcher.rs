use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use anyhow::{anyhow, Result};
use serde_json::Value;
use crate::paths;
use crate::skin_server;
use crate::versions::library_allowed;

fn load_version_json(id: &str) -> Result<Value> {
    let path = paths::versions_dir().join(id).join(format!("{id}.json"));
    Ok(serde_json::from_slice(&fs::read(&path)?)?)
}

fn maven_to_path(name: &str) -> Option<PathBuf> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let filename = if parts.len() >= 4 {
        format!("{artifact}-{version}-{}.jar", parts[3])
    } else {
        format!("{artifact}-{version}.jar")
    };
    Some(paths::libraries_dir().join(group).join(artifact).join(version).join(filename))
}

fn library_jar_path(lib: &Value) -> Option<PathBuf> {
    // 1. Пробуем стандартный путь Mojang
    if let Some(path) = lib.get("downloads")
        .and_then(|d| d.get("artifact"))
        .and_then(|a| a.get("path"))
        .and_then(|p| p.as_str()) 
    {
        return Some(paths::libraries_dir().join(path));
    }
    // 2. Если блока downloads нет (как у Fabric), парсим поле "name"
    let name = lib.get("name")?.as_str()?;
    maven_to_path(name)
}

fn native_jar_path(lib: &Value) -> Option<PathBuf> {
    if let Some(path) = lib.get("downloads")
        .and_then(|d| d.get("classifiers"))
        .and_then(|c| c.get("natives-linux"))
        .and_then(|n| n.get("path"))
        .and_then(|p| p.as_str()) 
    {
        return Some(paths::libraries_dir().join(path));
    }
    None
}

fn collect_game_args(json: &Value) -> Vec<String> {
    if let Some(s) = json.get("minecraftArguments").and_then(|v| v.as_str()) {
        return s.split_whitespace().map(|s| s.to_string()).collect();
    }
    json.get("arguments")
        .and_then(|a| a.get("game"))
        .and_then(|g| g.as_array())
        .map(|arr| arr.iter().filter_map(|i| i.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default()
}

struct Resolved {
    main_class: String,
    classpath: Vec<PathBuf>,
    asset_index: String,
    game_args: Vec<String>,
}

fn resolve(id: &str) -> Result<Resolved> {
    let json = load_version_json(id)?;
    let parent = json.get("inheritsFrom").and_then(|v| v.as_str()).map(|s| s.to_string());
    let parent_json = match &parent {
        Some(p) => Some(load_version_json(p)?),
        None => None,
    };

    let main_class = json["mainClass"]
        .as_str()
        .or_else(|| parent_json.as_ref().and_then(|p| p["mainClass"].as_str()))
        .ok_or_else(|| anyhow!("no mainClass"))?
        .to_string();

    let empty = Vec::new();
    let own_libs = json.get("libraries").and_then(|v| v.as_array()).unwrap_or(&empty);
    let parent_libs = parent_json.as_ref().and_then(|p| p.get("libraries")).and_then(|v| v.as_array()).unwrap_or(&empty);

    let mut classpath = Vec::new();
    
    // Создаем файл логов для отладки resolve
    let log_path = paths::game_dir().join("resolve_debug.log");
    let mut log = fs::File::create(&log_path)?;

    writeln!(log, "=== RESOLVE DEBUG for {} ===", id)?;
    writeln!(log, "Total own_libs: {}", own_libs.len())?;
    writeln!(log, "Total parent_libs: {}", parent_libs.len())?;

    for lib in parent_libs.iter().chain(own_libs.iter()) {
        let lib_name = lib.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
        
        if !library_allowed(lib) {
            writeln!(log, "[SKIP] {} (not allowed by rules)", lib_name)?;
            continue;
        }

        if let Some(p) = library_jar_path(lib) {
            if p.is_file() {
                classpath.push(p.clone());
                writeln!(log, "[OK] {} -> {}", lib_name, p.display())?;
            } else {
                writeln!(log, "[MISSING] {} -> {}", lib_name, p.display())?;
            }
        } else {
            writeln!(log, "[PARSE ERROR] {} (could not determine path)", lib_name)?;
        }

        if let Some(p) = native_jar_path(lib) {
            if p.is_file() {
                classpath.push(p.clone());
                writeln!(log, "[OK NATIVE] {} -> {}", lib_name, p.display())?;
            } else {
                writeln!(log, "[MISSING NATIVE] {} -> {}", lib_name, p.display())?;
            }
        }
    }

    let current_jar = paths::versions_dir().join(id).join(format!("{id}.jar"));
    if current_jar.is_file() {
        classpath.push(current_jar.clone());
        writeln!(log, "[OK CURRENT JAR] {}", current_jar.display())?;
    }

    if let Some(p) = &parent {
        let parent_jar = paths::versions_dir().join(p).join(format!("{p}.jar"));
        if parent_jar.is_file() {
            classpath.push(parent_jar.clone());
            writeln!(log, "[OK PARENT JAR] {}", parent_jar.display())?;
        }
    }

    writeln!(log, "Total classpath entries: {}", classpath.len())?;
    writeln!(log, "=== END RESOLVE DEBUG ===")?;

    let asset_index = json["assetIndex"]["id"]
        .as_str()
        .or_else(|| parent_json.as_ref().and_then(|p| p["assetIndex"]["id"].as_str()))
        .unwrap_or("legacy")
        .to_string();

    let mut game_args = collect_game_args(&json);
    if let Some(p) = &parent_json {
        for a in collect_game_args(p) {
            if !game_args.contains(&a) {
                game_args.push(a);
            }
        }
    }

    Ok(Resolved {
        main_class,
        classpath,
        asset_index,
        game_args,
    })
}

pub fn launch(id: &str, username: &str, uuid: &str) -> Result<()> {
    let resolved = resolve(id)?;
    fs::create_dir_all(paths::game_dir())?;

    skin_server::ensure_authlib_injector()?;
    let server = skin_server::SkinServer::start(username.to_string(), uuid.to_string())?;
    let port = server.port;
    std::mem::forget(server);

    let classpath = resolved
        .classpath
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(":");

    let mut ph = HashMap::new();
    ph.insert("${auth_player_name}", username.to_string());
    ph.insert("${version_name}", id.to_string());
    ph.insert("${game_directory}", paths::game_dir().to_string_lossy().to_string());
    ph.insert("${assets_root}", paths::assets_dir().to_string_lossy().to_string());
    ph.insert("${assets_index_name}", resolved.asset_index.clone());
    ph.insert("${auth_uuid}", uuid.to_string());
    ph.insert("${auth_access_token}", "0".to_string());
    ph.insert("${user_type}", "legacy".to_string());
    ph.insert("${version_type}", "release".to_string());
    ph.insert("${clientid}", String::new());
    ph.insert("${auth_xuid}", String::new());

    let mut java_args = Vec::new();
    java_args.push(format!(
        "-javaagent:{}={}",
        paths::authlib_injector_jar().to_string_lossy(),
        format!("http://127.0.0.1:{port}/api")
    ));
    java_args.push("-Dauthlibinjector.side=client".to_string());
    java_args.push(format!("-Djava.library.path={}", paths::libraries_dir().to_string_lossy()));
    java_args.push("-cp".to_string());
    java_args.push(classpath);
    java_args.push(resolved.main_class.clone());

    for arg in &resolved.game_args {
        let mut a = arg.clone();
        for (k, v) in &ph {
            a = a.replace(k, v);
        }
        java_args.push(a);
    }

    let args_file = paths::game_dir().join("launch_args.txt");
    let args_content = java_args.join("\n");
    fs::write(&args_file, &args_content)?;

    println!("Launching with argfile: {}", args_file.display());
    
    let mut cmd = Command::new("java");
    cmd.arg(format!("@{}", args_file.to_string_lossy()));
    cmd.current_dir(paths::game_dir());
    
    cmd.spawn()?;

    Ok(())
}