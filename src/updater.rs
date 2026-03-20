use anyhow::{anyhow, Context, Result};
use semver::Version;
use serde::Deserialize;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Sender;

pub const CURRENT_VERSION: &str = env!("LOGLINE_BUILD_VERSION");
pub const REPOSITORY_URL: &str = env!("CARGO_PKG_REPOSITORY");

const REPO_OWNER: &str = "zibo-chen";
const REPO_NAME: &str = "logline";

#[derive(Debug, Clone)]
pub struct ReleaseAsset {
    pub name: String,
    pub download_url: String,
}

#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    pub version: String,
    pub notes: String,
    pub html_url: String,
    pub published_at: Option<String>,
    pub asset: ReleaseAsset,
}

#[derive(Debug, Clone)]
pub struct PreparedUpdate {
    pub release: ReleaseInfo,
    pub file_path: PathBuf,
}

#[derive(Debug)]
pub enum UpdateEvent {
    CheckCompleted(Result<Option<ReleaseInfo>, String>),
    DownloadCompleted(Result<PreparedUpdate, String>),
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    body: String,
    html_url: String,
    published_at: Option<String>,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

pub fn start_check_for_updates(sender: Sender<UpdateEvent>) {
    std::thread::spawn(move || {
        let result = check_for_updates().map_err(|err| format!("{err:#}"));
        let _ = sender.send(UpdateEvent::CheckCompleted(result));
    });
}

pub fn start_download_update(release: ReleaseInfo, sender: Sender<UpdateEvent>) {
    std::thread::spawn(move || {
        let result = download_release_asset(release).map_err(|err| format!("{err:#}"));
        let _ = sender.send(UpdateEvent::DownloadCompleted(result));
    });
}

pub fn launch_installer(path: &Path) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new(path)
            .args([
                "/SP-",
                "/VERYSILENT",
                "/SUPPRESSMSGBOXES",
                "/NORESTART",
                "/CLOSEAPPLICATIONS",
                "/FORCECLOSEAPPLICATIONS",
            ])
            .spawn()
            .with_context(|| format!("Failed to launch installer: {}", path.display()))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .with_context(|| format!("Failed to open installer: {}", path.display()))?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        match Command::new("pkexec")
            .arg("dpkg")
            .arg("-i")
            .arg(path)
            .spawn()
        {
            Ok(_) => Ok(()),
            Err(pkexec_err) => {
                Command::new("xdg-open")
                    .arg(path)
                    .spawn()
                    .with_context(|| {
                        format!("Failed to launch installer with pkexec ({pkexec_err}) or xdg-open")
                    })?;
                Ok(())
            }
        }
    }
}

pub fn open_in_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .with_context(|| format!("Failed to open URL: {url}"))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .spawn()
            .with_context(|| format!("Failed to open URL: {url}"))?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .with_context(|| format!("Failed to open URL: {url}"))?;
        Ok(())
    }
}

fn check_for_updates() -> Result<Option<ReleaseInfo>> {
    let current = parse_version(CURRENT_VERSION)?;
    let release = fetch_latest_release()?;
    let latest = parse_version(&release.tag_name)?;

    if latest <= current {
        return Ok(None);
    }

    let asset = select_asset(&release.assets)
        .cloned()
        .ok_or_else(|| anyhow!("No installer asset found for the current platform"))?;

    Ok(Some(ReleaseInfo {
        version: latest.to_string(),
        notes: release.body.trim().to_string(),
        html_url: release.html_url,
        published_at: release.published_at,
        asset: ReleaseAsset {
            name: asset.name,
            download_url: asset.browser_download_url,
        },
    }))
}

fn fetch_latest_release() -> Result<GithubRelease> {
    let url = format!("https://api.github.com/repos/{REPO_OWNER}/{REPO_NAME}/releases/latest");
    reqwest::blocking::Client::builder()
        .user_agent(format!("{REPO_NAME}/{CURRENT_VERSION}"))
        .build()
        .context("Failed to build update client")?
        .get(url)
        .send()
        .context("Failed to query GitHub Releases API")?
        .error_for_status()
        .context("GitHub Releases API returned an error")?
        .json::<GithubRelease>()
        .context("Failed to parse GitHub release metadata")
}

fn download_release_asset(release: ReleaseInfo) -> Result<PreparedUpdate> {
    let download_dir = std::env::temp_dir()
        .join("logline-updates")
        .join(&release.version);
    fs::create_dir_all(&download_dir).context("Failed to create update cache directory")?;

    let file_path = download_dir.join(&release.asset.name);
    let mut response = reqwest::blocking::Client::builder()
        .user_agent(format!("{REPO_NAME}/{CURRENT_VERSION}"))
        .build()
        .context("Failed to build download client")?
        .get(&release.asset.download_url)
        .send()
        .with_context(|| format!("Failed to download {}", release.asset.name))?
        .error_for_status()
        .with_context(|| format!("GitHub rejected download for {}", release.asset.name))?;

    let mut file = File::create(&file_path)
        .with_context(|| format!("Failed to create {}", file_path.display()))?;
    io::copy(&mut response, &mut file)
        .with_context(|| format!("Failed to save {}", file_path.display()))?;

    Ok(PreparedUpdate { release, file_path })
}

fn parse_version(version: &str) -> Result<Version> {
    Version::parse(version.trim_start_matches('v'))
        .with_context(|| format!("Invalid version string: {version}"))
}

fn select_asset<'a>(assets: &'a [GithubAsset]) -> Option<&'a GithubAsset> {
    preferred_asset_patterns()
        .iter()
        .find_map(|pattern| assets.iter().find(|asset| asset.name.ends_with(pattern)))
}

fn preferred_asset_patterns() -> &'static [&'static str] {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        &["-windows-x86_64-setup.exe"]
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        &[
            "-macos-aarch64.dmg",
            "aarch64-apple-darwin.tar.gz",
            "aarch64-apple-darwin.tar.xz",
        ]
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        &[
            "-macos-x86_64.dmg",
            "x86_64-apple-darwin.tar.gz",
            "x86_64-apple-darwin.tar.xz",
        ]
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        &[
            "_amd64.deb",
            "x86_64-unknown-linux-gnu.tar.gz",
            "x86_64-unknown-linux-gnu.tar.xz",
        ]
    }

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        &[
            "_arm64.deb",
            "aarch64-unknown-linux-gnu.tar.gz",
            "aarch64-unknown-linux-gnu.tar.xz",
        ]
    }

    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64")
    )))]
    {
        &[]
    }
}
