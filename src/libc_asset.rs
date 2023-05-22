#[cfg(all(target_os = "linux", target_env = "musl"))]
#[cfg(target_arch = "x86_64")]
#[derive(rust_embed::RustEmbed)]
#[folder = "src/libc/x86_64/"]
struct Asset;

#[cfg(all(target_os = "linux", target_env = "musl"))]
#[cfg(target_arch = "aarch64")]
#[derive(rust_embed::RustEmbed)]
#[folder = "src/libc/aarch64/"]
struct Asset;

#[cfg(target_os = "linux")]
pub(crate) fn ld_env(envs: &mut std::collections::HashMap<String, String>) -> anyhow::Result<()> {
    use crate::{env, util};
    use anyhow::Context;
    use std::ops::Not;

    if is_musl()?.not() {
        log::debug!("[Asset] Run on glibc environment");
        return Ok(());
    }
    log::debug!("[Asset] Run on musl environment");

    let syno_lib_path = std::path::Path::new(env::SYNOPKG_LIB);
    if !syno_lib_path.exists() {
        std::fs::create_dir(&syno_lib_path).context(format!(
            "[Asset] Failed to create directory: {}",
            syno_lib_path.display()
        ))?;
    }
    for filename in Asset::iter()
        .map(|v| v.into_owned())
        .collect::<Vec<String>>()
    {
        let target_file = syno_lib_path.join(&filename);
        if !target_file.exists() {
            let file = Asset::get(&filename).context("[Asset] Failed to get bin asset")?;
            util::write_file(&target_file, file.data, 0o755)?;
        }
    }

    // patchelf handler
    #[cfg(feature = "patch")]
    {
        envs.insert(
            String::from("LD_LIBRARY_PATH"),
            env::SYNOPKG_LIB.to_string(),
        );
        log::info!("[Asset] LD_LIBRARY_PATH={}", env::SYNOPKG_LIB);
    }

    #[cfg(not(feature = "patch"))]
    {
        use std::ffi::CString;
        use std::path::Path;
        #[cfg(target_arch = "x86_64")]
        const LD: &str = "ld-linux-x86-64.so.2";
        #[cfg(target_arch = "aarch64")]
        const LD: &str = "ld-linux-aarch64.so.1";

        for sys_lib in env::SYS_LIB_ARRAY {
            let sys_lib_path = Path::new(sys_lib);
            let sys_ld_path = sys_lib_path.join(LD);
            let output = std::process::Command::new("ldd")
                .arg(env::LAUNCHER_EXE)
                .output()
                .expect("[Asset] Failed to execute ldd command");
            let stdout = String::from_utf8(output.stdout)?;
            log::debug!("[Asset] ldd stdout: {}", &stdout);
            match output.status.success()
                && stdout.contains(format!("{}", sys_ld_path.display()).as_str())
            {
                true => {
                    if sys_lib_path.exists().not() {
                        util::create_dir_all(&sys_lib_path, 0o755)?
                    }
                    // Compatible MUSL systems may come with libc
                    if sys_ld_path.exists() {
                        let real_ld_path = std::fs::canonicalize(&sys_ld_path)?;
                        let real_lib_path = real_ld_path.parent().context(format!(
                            "[Asset] The library path does not exist: {}",
                            real_ld_path.display()
                        ))?;
                        log::info!(
                            "[Asset] Real path of the symlink {}: {}",
                            sys_ld_path.display(),
                            real_ld_path.display()
                        );
                        envs.insert(
                            String::from("LD_LIBRARY_PATH"),
                            format!("{}", real_lib_path.display()),
                        );
                        log::info!(
                            "[Asset] LD_LIBRARY_PATH={}",
                            format!("{}", real_lib_path.display())
                        );
                        return Ok(());
                    }
                    let syno_ld_path = Path::new(env::SYNOPKG_LIB).join(LD);
                    unsafe {
                        let source_path = CString::new(syno_ld_path.display().to_string())?;
                        let target_path = CString::new(sys_ld_path.display().to_string())?;
                        if libc::symlink(source_path.as_ptr(), target_path.as_ptr()) != 0 {
                            anyhow::bail!(std::io::Error::last_os_error());
                        }
                    }
                    envs.insert(
                        String::from("LD_LIBRARY_PATH"),
                        env::SYNOPKG_LIB.to_string(),
                    );
                    log::info!("[Asset] LD_LIBRARY_PATH={}", env::SYNOPKG_LIB);
                    return Ok(());
                }
                false => {}
            }
        }
    }
    Ok(())
}

fn is_musl() -> anyhow::Result<bool> {
    let output = std::process::Command::new("sh")
        .args(["-c", "ldd --version"])
        .output()
        .unwrap();
    let out = match output.status.success() {
        true => String::from_utf8(output.stdout).unwrap(),
        false => String::from_utf8(output.stderr).unwrap(),
    };
    log::debug!("[Asset] ldd --version stdout: {}", out);
    Ok(out.to_lowercase().contains("musl"))
}
