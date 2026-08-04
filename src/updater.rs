use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;

const GITHUB_REPO_OWNER: &str = "BingFengHung";
const GITHUB_REPO_NAME: &str = "cpl";

#[derive(Deserialize, Debug)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize, Debug)]
struct Release {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

pub fn check_and_update(force: bool) -> Result<()> {
    let current_ver = env!("CARGO_PKG_VERSION");
    println!("🔍 Current version: v{}", current_ver);
    println!("🌐 Checking for updates on GitHub Releases...");

    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        GITHUB_REPO_OWNER, GITHUB_REPO_NAME
    );

    let response: Release = ureq::get(&url)
        .set("User-Agent", "cpl-updater")
        .call()
        .map_err(|e| anyhow!("Failed to query GitHub Releases API: {}", e))?
        .into_json()
        .map_err(|e| anyhow!("Failed to parse release JSON: {}", e))?;

    let latest_ver = response.tag_name.trim_start_matches('v');
    println!("📦 Latest version found: v{}", latest_ver);

    let is_newer = match (semver::Version::parse(current_ver), semver::Version::parse(latest_ver)) {
        (Ok(curr), Ok(latest)) => latest > curr,
        _ => latest_ver != current_ver,
    };

    if !is_newer && !force {
        println!("✨ You are already using the latest version of cpl (v{})!", current_ver);
        return Ok(());
    }

    println!("🚀 Updating cpl to v{}...", latest_ver);

    let target_asset_name = get_target_asset_name();
    let asset = response
        .assets
        .iter()
        .find(|a| a.name.contains(&target_asset_name) || a.name == target_asset_name)
        .ok_or_else(|| {
            anyhow!(
                "No prebuilt binary asset found for your platform ({}) in release v{}",
                target_asset_name,
                latest_ver
            )
        })?;

    println!("📥 Downloading asset: {}...", asset.name);

    let resp = ureq::get(&asset.browser_download_url)
        .set("User-Agent", "cpl-updater")
        .call()
        .map_err(|e| anyhow!("Failed to download release asset: {}", e))?;

    let mut binary_bytes = Vec::new();
    std::io::copy(&mut resp.into_reader(), &mut binary_bytes)
        .context("Failed to read downloaded binary stream")?;

    let current_exe_path = env::current_exe().context("Could not determine current executable path")?;
    replace_current_binary(&current_exe_path, &binary_bytes)?;

    println!("🎉 Success! cpl has been updated to v{}!", latest_ver);
    Ok(())
}

fn get_target_asset_name() -> String {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    match (os, arch) {
        ("windows", _) => "windows".to_string(),
        ("macos", "aarch64") => "macos-arm64".to_string(),
        ("macos", _) => "macos".to_string(),
        ("linux", _) => "linux".to_string(),
        _ => os.to_string(),
    }
}

fn replace_current_binary(current_exe: &PathBuf, new_binary_bytes: &[u8]) -> Result<()> {
    let temp_path = current_exe.with_extension("tmp");
    let old_path = current_exe.with_extension("old");

    fs::write(&temp_path, new_binary_bytes).context("Failed to write new binary to temp file")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o755))?;
    }

    if old_path.exists() {
        let _ = fs::remove_file(&old_path);
    }

    // Rename current exe to .old, then rename temp file to current exe
    if let Err(_) = fs::rename(current_exe, &old_path) {
        // If rename fails (e.g. permission or lock), try overwrite directly
        fs::write(current_exe, new_binary_bytes)?;
    } else {
        fs::rename(&temp_path, current_exe)?;
        let _ = fs::remove_file(&old_path);
    }

    if temp_path.exists() {
        let _ = fs::remove_file(temp_path);
    }

    Ok(())
}
