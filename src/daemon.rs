extern crate libc;

use std::ops::Not;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use rand::Rng;

use crate::env;

use crate::util;
use crate::xunlei_asset;
use crate::xunlei_asset::Xunlei;

use crate::Config;
use crate::Running;

pub struct XunleiInstall {
    description: &'static str,
    auth_user: Option<String>,
    auth_password: Option<String>,
    host: std::net::IpAddr,
    port: u16,
    download_path: PathBuf,
    config_path: PathBuf,
    uid: u32,
    gid: u32,
}

impl From<Config> for XunleiInstall {
    fn from(config: Config) -> Self {
        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };
        Self {
            description: "Thunder remote download service",
            host: config.host,
            port: config.port,
            download_path: config.download_path,
            config_path: config.config_path,
            uid,
            gid,
            auth_user: config.auth_user,
            auth_password: config.auth_password,
        }
    }
}

impl XunleiInstall {
    fn config(&self) -> anyhow::Result<()> {
        log::info!("[XunleiInstall] Configuration in progress");
        log::info!("[XunleiInstall] WebUI port: {}", self.port);

        if self.download_path.is_dir().not() {
            std::fs::create_dir_all(&self.download_path)?;
        } else if self.download_path.is_file() {
            return Err(anyhow::anyhow!("Download path must be a directory"));
        }

        if self.config_path.is_dir().not() {
            std::fs::create_dir_all(&self.config_path)?;
        } else if self.config_path.is_file() {
            return Err(anyhow::anyhow!("Config path must be a directory"));
        }
        log::info!(
            "[XunleiInstall] Config directory: {}",
            self.config_path.display()
        );
        log::info!(
            "[XunleiInstall] Download directory: {}",
            self.download_path.display()
        );
        log::info!("[XunleiInstall] Configuration completed");
        Ok(())
    }

    fn install(&self) -> anyhow::Result<std::path::PathBuf> {
        log::info!("[XunleiInstall] Installing in progress");

        // /var/packages/pan-xunlei-com/target
        let target_dir = PathBuf::from(env::SYNOPKG_PKGDEST);
        // /var/packages/pan-xunlei-com/target/host
        let host_dir = PathBuf::from(env::SYNOPKG_HOST);

        util::create_dir_all(&target_dir, 0o755)?;

        let xunlei = xunlei_asset::asset()?;
        for file in xunlei.iter()? {
            let filename = file.as_str();
            let target_filepath = target_dir.join(filename);
            let data = xunlei.get(filename).context("Read data failure")?;
            util::write_file(&target_filepath, data, 0o755)?;
            log::info!("[XunleiInstall] Install to: {}", target_filepath.display());
        }

        util::set_permissions(env::SYNOPKG_PKGBASE, self.uid, self.gid).context(format!(
            "Failed to set permission: {}, PUID:{}, GUID:{}",
            env::SYNOPKG_PKGBASE,
            self.uid,
            self.gid
        ))?;

        util::set_permissions(target_dir.to_str().unwrap(), self.uid, self.gid).context(
            format!(
                "Failed to set permission: {}, PUID:{}, GUID:{}",
                target_dir.display(),
                self.uid,
                self.gid
            ),
        )?;

        // path: /var/packages/pan-xunlei-com/target/host/etc/synoinfo.conf
        let syno_info_path =
            PathBuf::from(format!("{}{}", host_dir.display(), env::SYNO_INFO_PATH));
        util::create_dir_all(
            syno_info_path.parent().context(format!(
                "the path: {} parent not exists",
                syno_info_path.display()
            ))?,
            0o755,
        )?;
        let mut byte_arr = vec![0u8; 32];
        rand::thread_rng().fill(&mut byte_arr[..]);
        let hex_string = byte_arr
            .iter()
            .map(|u| format!("{:02x}", *u as u32))
            .collect::<String>()
            .chars()
            .take(7)
            .collect::<String>();
        util::write_file(
            &syno_info_path,
            std::borrow::Cow::Borrowed(
                format!("unique=\"synology_{}_720+\"", hex_string).as_bytes(),
            ),
            0o644,
        )?;

        // path: /var/packages/pan-xunlei-com/target/host/usr/syno/synoman/webman/modules/authenticate.cgi
        let syno_authenticate_path = PathBuf::from(format!(
            "{}{}",
            host_dir.display(),
            env::SYNO_AUTHENTICATE_PATH
        ));
        util::create_dir_all(
            syno_authenticate_path.parent().context(format!(
                "directory path: {} not exists",
                syno_authenticate_path.display()
            ))?,
            0o755,
        )?;
        util::write_file(
            &syno_authenticate_path,
            std::borrow::Cow::Borrowed(String::from("#!/usr/bin/env sh\necho OK").as_bytes()),
            0o755,
        )?;

        // symlink
        unsafe {
            if Path::new(env::SYNO_INFO_PATH).exists().not() {
                let source_sys_info_path =
                    std::ffi::CString::new(syno_info_path.display().to_string())?;
                let target_sys_info_path = std::ffi::CString::new(env::SYNO_INFO_PATH)?;
                if libc::symlink(source_sys_info_path.as_ptr(), target_sys_info_path.as_ptr()) != 0
                {
                    anyhow::bail!(std::io::Error::last_os_error());
                }
            }

            let link_syno_authenticate_path = Path::new(env::SYNO_AUTHENTICATE_PATH);
            if link_syno_authenticate_path.exists().not() {
                let source_syno_authenticate_path =
                    std::ffi::CString::new(syno_authenticate_path.display().to_string())?;
                let target_syno_authenticate_path =
                    std::ffi::CString::new(env::SYNO_AUTHENTICATE_PATH)?;
                let patent_ = link_syno_authenticate_path.parent().context(format!(
                    "directory path: {} not exists",
                    link_syno_authenticate_path.display()
                ))?;
                util::create_dir_all(patent_, 0o755)?;
                if libc::symlink(
                    source_syno_authenticate_path.as_ptr(),
                    target_syno_authenticate_path.as_ptr(),
                ) != 0
                {
                    anyhow::bail!(std::io::Error::last_os_error());
                }
            }
        }

        log::info!("[XunleiInstall] Installation completed");
        Ok(std::env::current_exe()?)
    }

    fn systemd(&self, launcher: PathBuf) -> anyhow::Result<()> {
        if Systemd::support().not() {
            return Ok(());
        }

        let auth = match self.auth_user.is_some() && self.auth_password.is_some() {
            true => format!(
                "-U {} -W {}",
                self.auth_user.clone().unwrap_or_default(),
                self.auth_password.clone().unwrap_or_default()
            ),
            false => "".to_string(),
        };

        let systemctl_unit = format!(
            r#"[Unit]
                Description={}
                After=network.target network-online.target
                Requires=network-online.target
                
                [Service]
                Type=simple
                ExecStart={} launcher -h {} -p {} -d {} -c {} {}
                LimitNOFILE=1024
                LimitNPROC=512
                User={}
                
                [Install]
                WantedBy=multi-user.target"#,
            self.description,
            launcher.display(),
            self.host,
            self.port,
            self.download_path.display(),
            self.config_path.display(),
            auth,
            self.uid
        );

        util::write_file(
            &PathBuf::from(env::SYSTEMCTL_UNIT_FILE),
            std::borrow::Cow::Borrowed(systemctl_unit.as_bytes()),
            0o666,
        )?;

        Systemd::systemctl(["daemon-reload"])?;
        Systemd::systemctl(["enable", env::APP_NAME])?;
        Systemd::systemctl(["start", env::APP_NAME])?;
        Ok(())
    }
}

impl Running for XunleiInstall {
    fn run(self) -> anyhow::Result<()> {
        self.config()?;
        self.systemd(self.install()?)
    }
}

pub struct XunleiUninstall {
    clear: bool,
}

impl XunleiUninstall {
    fn uninstall(&self) -> anyhow::Result<()> {
        if Systemd::support() {
            let path = Path::new(env::SYSTEMCTL_UNIT_FILE);
            if path.exists() {
                std::fs::remove_file(path)?;
                log::info!("[XunleiUninstall] Uninstall xunlei service");
            }
        }
        let path = Path::new(env::SYNOPKG_PKGBASE);
        if path.exists() {
            std::fs::remove_dir_all(path)?;
            log::info!("[XunleiUninstall] Uninstall xunlei package");
        }

        // Clear xunlei default config directory
        if self.clear {
            let path = Path::new(env::DEFAULT_CONFIG_PATH);
            if path.exists() {
                std::fs::remove_dir_all(Path::new(path))?
            }
        }

        Ok(())
    }
}

impl Running for XunleiUninstall {
    fn run(self) -> anyhow::Result<()> {
        if Systemd::support() {
            Systemd::systemctl(["disable", env::APP_NAME])?;
            Systemd::systemctl(["stop", env::APP_NAME])?;
            Systemd::systemctl(["daemon-reload"])?;
        }
        self.uninstall()?;
        Ok(())
    }
}

impl From<bool> for XunleiUninstall {
    fn from(value: bool) -> Self {
        XunleiUninstall { clear: value }
    }
}

struct Systemd;

impl Systemd {
    fn support() -> bool {
        let child_res = std::process::Command::new("systemctl")
            .arg("--help")
            .output();

        let support = match child_res {
            Ok(output) => output.status.success(),
            Err(_) => false,
        };
        if support.not() {
            log::warn!("[Systemd] Your system does not support systemctl");
        }
        support
    }

    fn systemctl<I, S>(args: I) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr> + std::convert::AsRef<std::ffi::OsStr>,
    {
        let output = std::process::Command::new("systemctl")
            .args(args)
            .output()?;
        if output.status.success().not() {
            log::error!(
                "[systemctl] {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(())
    }
}
