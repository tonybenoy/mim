use crate::Result;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use tracing::{info, warn};

const SYNCTHING_VERSION: &str = "1.29.2";

/// Manages an embedded Syncthing instance as a sidecar process.
/// Syncthing is downloaded once, configured automatically, and runs
/// invisibly in the background for phone-to-PC photo sync.
pub struct SyncManager {
    bin_path: PathBuf,
    config_dir: PathBuf,
    process: Option<Child>,
    api_key: String,
    api_port: u16,
}

impl SyncManager {
    pub fn new(data_dir: &Path) -> Self {
        let sync_dir = data_dir.join("syncthing");
        Self {
            bin_path: sync_dir.join("syncthing"),
            config_dir: sync_dir.join("config"),
            process: None,
            api_key: uuid::Uuid::new_v4().to_string().replace('-', ""),
            api_port: 8384,
        }
    }

    /// Check if the Syncthing binary is available.
    pub fn is_installed(&self) -> bool {
        self.bin_path.exists()
    }

    /// Download the Syncthing binary for the current platform.
    pub async fn ensure_binary(&self) -> Result<()> {
        if self.is_installed() {
            info!("Syncthing binary already present");
            return Ok(());
        }

        let parent = self.bin_path.parent().unwrap();
        std::fs::create_dir_all(parent)?;

        let (os, arch, ext) = platform_info();
        let url = format!(
            "https://github.com/syncthing/syncthing/releases/download/v{}/syncthing-{}-{}-v{}.tar.gz",
            SYNCTHING_VERSION, os, arch, SYNCTHING_VERSION
        );

        info!("Downloading Syncthing from {}", url);

        let response = reqwest::get(&url)
            .await
            .map_err(|e| crate::error::Error::Other(format!("download: {e}")))?;

        if !response.status().is_success() {
            return Err(crate::error::Error::Other(format!(
                "Syncthing download failed: HTTP {}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| crate::error::Error::Other(format!("read body: {e}")))?;

        // Extract the binary from the tarball
        let decoder = flate2::read::GzDecoder::new(&bytes[..]);
        let mut archive = tar::Archive::new(decoder);

        for entry in archive
            .entries()
            .map_err(|e| crate::error::Error::Other(format!("tar: {e}")))?
        {
            let mut entry =
                entry.map_err(|e| crate::error::Error::Other(format!("tar entry: {e}")))?;
            let path = entry
                .path()
                .map_err(|e| crate::error::Error::Other(format!("tar path: {e}")))?
                .to_path_buf();

            // Find the syncthing binary in the archive
            if let Some(name) = path.file_name() {
                if name == "syncthing" || name == "syncthing.exe" {
                    entry
                        .unpack(&self.bin_path)
                        .map_err(|e| crate::error::Error::Other(format!("unpack: {e}")))?;

                    // Make executable on Unix
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        std::fs::set_permissions(
                            &self.bin_path,
                            std::fs::Permissions::from_mode(0o755),
                        )?;
                    }

                    info!("Syncthing binary installed at {}", self.bin_path.display());
                    return Ok(());
                }
            }
        }

        Err(crate::error::Error::Other(
            "Syncthing binary not found in archive".into(),
        ))
    }

    /// Start the Syncthing process in the background.
    pub fn start(&mut self) -> Result<()> {
        if self.process.is_some() {
            return Ok(());
        }

        std::fs::create_dir_all(&self.config_dir)?;

        // Write a minimal config if none exists
        let config_file = self.config_dir.join("config.xml");
        if !config_file.exists() {
            self.write_initial_config(&config_file)?;
        }

        let child = Command::new(&self.bin_path)
            .arg("serve")
            .arg("--no-browser")
            .arg("--no-restart")
            .arg(format!("--home={}", self.config_dir.display()))
            .arg(format!("--gui-address=127.0.0.1:{}", self.api_port))
            .arg(format!("--gui-apikey={}", self.api_key))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| crate::error::Error::Other(format!("spawn syncthing: {e}")))?;

        info!("Syncthing started (PID: {})", child.id());
        self.process = Some(child);
        Ok(())
    }

    /// Stop the Syncthing process.
    pub fn stop(&mut self) {
        if let Some(ref mut child) = self.process {
            let _ = child.kill();
            let _ = child.wait();
            info!("Syncthing stopped");
        }
        self.process = None;
    }

    /// Check if the process is running.
    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.process {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.process = None;
                    false
                }
                Ok(None) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// Get the local Syncthing device ID for QR code display.
    pub async fn get_device_id(&self) -> Result<String> {
        let url = format!(
            "http://127.0.0.1:{}/rest/system/status",
            self.api_port
        );

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .map_err(|e| crate::error::Error::Other(format!("api: {e}")))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| crate::error::Error::Other(format!("json: {e}")))?;

        body["myID"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| crate::error::Error::Other("no device ID".into()))
    }

    /// Add a shared folder that phones can sync to.
    pub async fn add_sync_folder(&self, folder_path: &str, label: &str) -> Result<()> {
        let folder_id = format!("mim-{}", &uuid::Uuid::new_v4().to_string()[..8]);

        let folder_config = serde_json::json!({
            "id": folder_id,
            "label": label,
            "path": folder_path,
            "type": "receiveonly",
            "rescanIntervalS": 60,
            "fsWatcherEnabled": true,
            "fsWatcherDelayS": 10,
        });

        let url = format!(
            "http://127.0.0.1:{}/rest/config/folders",
            self.api_port
        );

        let client = reqwest::Client::new();
        client
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .json(&folder_config)
            .send()
            .await
            .map_err(|e| crate::error::Error::Other(format!("add folder: {e}")))?;

        info!("Added sync folder: {} -> {}", folder_id, folder_path);
        Ok(())
    }

    /// Get the API port for the GUI/REST interface.
    pub fn api_port(&self) -> u16 {
        self.api_port
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    fn write_initial_config(&self, path: &Path) -> Result<()> {
        let config = format!(
            r#"<configuration version="37">
    <gui enabled="true" tls="false" debugging="false">
        <address>127.0.0.1:{port}</address>
        <apikey>{key}</apikey>
        <theme>dark</theme>
    </gui>
    <options>
        <listenAddress>default</listenAddress>
        <globalAnnounceEnabled>true</globalAnnounceEnabled>
        <localAnnounceEnabled>true</localAnnounceEnabled>
        <relaysEnabled>true</relaysEnabled>
        <urAccepted>-1</urAccepted>
        <autoUpgradeIntervalH>0</autoUpgradeIntervalH>
    </options>
</configuration>"#,
            port = self.api_port,
            key = self.api_key,
        );
        std::fs::write(path, config)?;
        Ok(())
    }
}

impl Drop for SyncManager {
    fn drop(&mut self) {
        self.stop();
    }
}

fn platform_info() -> (&'static str, &'static str, &'static str) {
    let os = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "amd64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "arm") {
        "arm"
    } else {
        "amd64"
    };

    let ext = if cfg!(target_os = "windows") {
        "zip"
    } else {
        "tar.gz"
    };

    (os, arch, ext)
}
