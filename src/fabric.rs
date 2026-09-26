use crate::paths;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

pub fn fetch_loader_versions(mc_version: &str) -> Result<Vec<String>> {
    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{mc_version}");
    let body: Value = ureq::get(&url).call()?.into_json()?;
    let arr = body.as_array().ok_or_else(|| anyhow!("bad fabric response"))?;
    Ok(arr
        .iter()
        .filter_map(|e| e["loader"]["version"].as_str().map(|s| s.to_string()))
        .collect())
}

pub fn install_fabric(mc_version: &str, loader_version: &str, progress: &dyn Fn(String)) -> Result<String> {
    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{mc_version}/{loader_version}/profile/json");
    progress("fetching fabric profile".to_string());
    let profile: Value = ureq::get(&url).call()?.into_json()?;
    let id = profile["id"].as_str().ok_or_else(|| anyhow!("bad fabric profile"))?.to_string();

    let version_dir = paths::versions_dir().join(&id);
    fs::create_dir_all(&version_dir)?;
    fs::write(version_dir.join(format!("{id}.json")), serde_json::to_vec_pretty(&profile)?)?;

    // 1. Сначала скачиваем/проверяем родительскую ванильную версию Minecraft (например, 1.20.4)
    if let Some(inherits) = profile.get("inheritsFrom").and_then(|v| v.as_str()) {
        progress(format!("checking parent vanilla version: {inherits}"));
        crate::versions::install_version(inherits, progress)?;
    }

    // 2. Затем скачиваем собственные библиотеки Fabric
    if let Some(libs) = profile.get("libraries") {
        progress("downloading fabric libraries".to_string());
        download_libraries(libs, progress)?;
    }

    progress(format!("fabric {loader_version} for {mc_version}: install complete"));
    Ok(id)
}

