use rouille::router;
use rouille::Request;
use rouille::Response;
use std::collections::HashMap;
use std::io;
use std::sync::Mutex;

use anyhow::Context;
use signal_hook::iterator::Signals;

use crate::util;
use crate::{env, Config, Running};
use std::{
    io::Read,
    ops::Not,
    path::{Path, PathBuf},
    process::Stdio,
};

const HTML_LOGIN: &str = include_str!("../static/login.html");
const JS_SHA3: &str = include_str!("../static/sha3.min.js");

// hasher auth message
fn hasher_auth_message(s: &str) -> String {
    use sha3::{Digest, Sha3_512};
    let mut hasher = Sha3_512::new();
    hasher.update(s);
    format!("{:x}", hasher.finalize())
}

#[derive(Clone)]
pub struct XunleiLauncher {
    auth_user: Option<String>,
    auth_password: Option<String>,
    host: std::net::IpAddr,
    port: u16,
    download_path: PathBuf,
    config_path: PathBuf,
    mount_bind_download_path: PathBuf,
}

impl From<Config> for XunleiLauncher {
    fn from(config: Config) -> Self {
        let auth_user = config
            .auth_user
            .map(|auth_user| hasher_auth_message(auth_user.as_str()));

        let auth_password = config
            .auth_password
            .map(|auth_password| hasher_auth_message(auth_password.as_str()));
        Self {
            auth_user,
            auth_password,
            host: config.host,
            port: config.port,
            download_path: config.download_path,
            config_path: config.config_path,
            mount_bind_download_path: config.mount_bind_download_path,
        }
    }
}

impl XunleiLauncher {
    fn envs(&self) -> anyhow::Result<HashMap<String, String>> {
        let mut envs = HashMap::new();
        envs.insert(String::from("DriveListen"), String::from(env::SOCK_FILE));
        envs.insert(
            String::from("OS_VERSION"),
            format!(
                "dsm {}.{}-{}",
                env::SYNOPKG_DSM_VERSION_MAJOR,
                env::SYNOPKG_DSM_VERSION_MINOR,
                env::SYNOPKG_DSM_VERSION_BUILD
            ),
        );
        envs.insert(String::from("HOME"), self.config_path.display().to_string());
        envs.insert(
            String::from("ConfigPath"),
            self.config_path.display().to_string(),
        );
        envs.insert(
            String::from("DownloadPATH"),
            self.mount_bind_download_path.display().to_string(),
        );
        envs.insert(
            String::from("SYNOPKG_DSM_VERSION_MAJOR"),
            String::from(env::SYNOPKG_DSM_VERSION_MAJOR),
        );
        envs.insert(
            String::from("SYNOPKG_DSM_VERSION_MINOR"),
            String::from(env::SYNOPKG_DSM_VERSION_MINOR),
        );
        envs.insert(
            String::from("SYNOPKG_DSM_VERSION_BUILD"),
            String::from(env::SYNOPKG_DSM_VERSION_BUILD),
        );

        envs.insert(
            String::from("SYNOPKG_PKGDEST"),
            String::from(env::SYNOPKG_PKGDEST),
        );
        envs.insert(
            String::from("SYNOPKG_PKGNAME"),
            String::from(env::SYNOPKG_PKGNAME),
        );
        envs.insert(String::from("SVC_CWD"), String::from(env::SYNOPKG_PKGDEST));

        envs.insert(String::from("PID_FILE"), String::from(env::PID_FILE));
        envs.insert(String::from("ENV_FILE"), String::from(env::ENV_FILE));
        envs.insert(String::from("LOG_FILE"), String::from(env::LOG_FILE));
        envs.insert(
            String::from("LAUNCH_LOG_FILE"),
            String::from(env::LAUNCH_LOG_FILE),
        );
        envs.insert(
            String::from("LAUNCH_PID_FILE"),
            String::from(env::LAUNCH_PID_FILE),
        );
        envs.insert(String::from("INST_LOG"), String::from(env::INST_LOG));
        envs.insert(String::from("GIN_MODE"), String::from("release"));

        #[cfg(all(target_os = "linux", target_env = "musl"))]
        crate::libc_asset::ld_env(&mut envs)?;
        Ok(envs)
    }
}

impl Running for XunleiLauncher {
    fn run(self) -> anyhow::Result<()> {
        use std::thread::{Builder, JoinHandle};

        let envs = self.envs()?;

        let args = (
            self.download_path.clone(),
            self.mount_bind_download_path.clone(),
            envs.clone(),
        );
        let backend_thread: JoinHandle<_> = Builder::new()
            .name("backend".to_string())
            .spawn(move || match XunleiBackendServer::from(args).run() {
                Ok(_) => {}
                Err(e) => log::error!("[XunleiBackendServer] error: {}", e),
            })
            .expect("[XunleiLauncher] Failed to start backend thread");

        let args = (self, envs);
        std::thread::spawn(move || match XunleiPanelServer::from(args).run() {
            Ok(_) => {}
            Err(e) => log::error!("[XunleiPanelServer] error: {}", e),
        });

        backend_thread
            .join()
            .expect("[XunleiLauncher] Failed to join thread");

        log::info!("[XunleiLauncher] All services have been complete");
        Ok(())
    }
}

use libc::{mount, umount2, MNT_DETACH, MS_BIND};
use std::ffi::CString;
use std::os::raw::c_int;
use std::ptr;

struct XunleiBackendServer {
    download_path: PathBuf,
    mount_bind_download_path: PathBuf,
    envs: HashMap<String, String>,
}

impl XunleiBackendServer {
    fn bind_mount(source: &Path, target: &Path) -> c_int {
        if Self::umount(target) == 0 {
            log::info!(
                "[XunleiBackendServer] Unmount {} succeeded.",
                target.display()
            )
        }
        let source_cstr =
            CString::new(format!("{}", source.display())).expect("source CString new error");
        let target_cstr =
            CString::new(format!("{}", target.display())).expect("target CString new error");
        unsafe {
            mount(
                source_cstr.as_ptr(),
                target_cstr.as_ptr(),
                ptr::null(),
                MS_BIND,
                ptr::null(),
            )
        }
    }

    fn umount(target: &Path) -> c_int {
        let target_cstr =
            CString::new(format!("{}", target.display())).expect("target CString new error");
        unsafe { umount2(target_cstr.as_ptr(), MNT_DETACH) }
    }
}

impl Running for XunleiBackendServer {
    fn run(self) -> anyhow::Result<()> {
        let var_path = Path::new(env::SYNOPKG_VAR);
        if var_path.exists().not() {
            util::create_dir_all(var_path, 0o777)?;
        }

        // mount bind downloads directory
        if self.mount_bind_download_path.exists().not() {
            util::create_dir_all(&self.mount_bind_download_path, 0o755)?;
        }

        // the real store download path
        if self.download_path.exists().not() {
            util::create_dir_all(&self.download_path, 0o755)?;
        }

        match Self::bind_mount(&self.download_path, &self.mount_bind_download_path) {
            0 => log::info!(
                "[XunleiBackendServer] Mount {} to {} succeeded",
                self.download_path.display(),
                self.mount_bind_download_path.display()
            ),
            _ => anyhow::bail!(
                "[XunleiBackendServer] Mount {} to {} failed",
                self.download_path.display(),
                self.mount_bind_download_path.display()
            ),
        }

        log::info!("[XunleiBackendServer] Start Xunlei Backend Server");
        let backend_process = std::process::Command::new(env::LAUNCHER_EXE)
            .args([
                format!("-launcher_listen={}", env::LAUNCHER_SOCK),
                format!("-pid={}", env::PID_FILE),
                format!("-logfile={}", env::LAUNCH_LOG_FILE),
                #[cfg(feature = "patch")]
                format!("-update_url={}", "empty"),
            ])
            .current_dir(env::SYNOPKG_PKGDEST)
            .envs(self.envs)
            // Join the parent process group by default
            .spawn()
            .expect("failed to spawn child process");
        let backend_pid = backend_process.id() as libc::pid_t;
        log::info!(
            "[XunleiBackendServer] Xunlei Backend Server PID: {}",
            backend_pid
        );

        let mut signals = Signals::new([
            signal_hook::consts::SIGINT,
            signal_hook::consts::SIGHUP,
            signal_hook::consts::SIGTERM,
        ])?;

        for signal in signals.forever() {
            match signal {
                signal_hook::consts::SIGINT
                | signal_hook::consts::SIGHUP
                | signal_hook::consts::SIGTERM => {
                    unsafe { libc::kill(backend_pid, libc::SIGINT) };
                    log::info!("[XunleiBackendServer] The backend service has been terminated");
                    break;
                }
                _ => {
                    log::warn!("[XunleiBackendServer] The system receives an unprocessed signal")
                }
            }
        }

        // umount bind directory
        match Self::umount(&self.mount_bind_download_path) {
            0 => log::info!(
                "[XunleiBackendServer] Unmount {} succeeded",
                self.mount_bind_download_path.display()
            ),
            _ => log::error!(
                "[XunleiBackendServer] Unmount {} failed",
                self.mount_bind_download_path.display()
            ),
        }

        Ok(())
    }
}

impl From<(PathBuf, PathBuf, HashMap<String, String>)> for XunleiBackendServer {
    fn from(value: (PathBuf, PathBuf, HashMap<String, String>)) -> Self {
        Self {
            download_path: value.0,
            mount_bind_download_path: value.1,
            envs: value.2,
        }
    }
}

// This struct contains the data that we store on the server about each client.
#[derive(Debug, Clone)]
struct Session;

#[macro_export]
macro_rules! try_or_400 {
    ($result:expr) => {
        match $result {
            Ok(r) => r,
            Err(err) => {
                let json = rouille::try_or_400::ErrJson::from_err(&err);
                return Ok(rouille::Response::json(&json).with_status_code(400));
            }
        }
    };
}

struct XunleiPanelServer {
    auth_user: Option<String>,
    auth_password: Option<String>,
    host: std::net::IpAddr,
    port: u16,
    envs: HashMap<String, String>,
}

impl XunleiPanelServer {
    fn authentication(&self, auth_user: String, auth_password: String) -> bool {
        let raw_auth_user = self.auth_user.clone().unwrap_or_default();
        let raw_auth_password = self.auth_password.clone().unwrap_or_default();
        auth_user.eq(&raw_auth_user) && auth_password.eq(&raw_auth_password)
    }

    #[allow(unreachable_code)]
    fn handle_route(
        &self,
        request: &Request,
        session_data: &mut Option<Session>,
    ) -> anyhow::Result<Response> {
        if self.auth_user.is_none() || self.auth_password.is_none() {
            *session_data = Some(Session {});
        }

        rouille::router!(request,
            (POST) (/login) => {
                let data = try_or_400!(rouille::post_input!(request, {
                    auth_user: String,
                    auth_password: String,
                }));
                if self.authentication(data.auth_user, data.auth_password) {
                    *session_data = Some(Session{});
                    return Ok(Response::redirect_303("/"));
                } else {
                    return Ok(Response::html("Wrong login/password"));
                }
            },
            _ => ()
        );

        if let Some(_session_data) = session_data.as_ref() {
            // Logged in.
            self.handle_route_logged_in(request)
        } else {
            // Not logged in.
            router!(request,
                (GET) ["/login"] => {
                    Ok(Response::html(HTML_LOGIN))
                },
                (GET) ["/js/sha3.min.js"] => {
                    Ok(Response::html(JS_SHA3))
                },
                _ => {
                    Ok(Response::redirect_303("/login"))
                }
            )
        }
    }

    // This function handles the routes that are accessible only if the user is logged in.
    fn handle_route_logged_in(&self, request: &Request) -> anyhow::Result<Response> {
        rouille::router!(request,
            (GET) ["/webman/login.cgi"] => {
                Ok(rouille::Response::json(&String::from(r#"{"SynoToken", ""}"#)).with_additional_header("Content-Type","application/json; charset=utf-8").with_status_code(200))
             },
            _ => {
                if request.raw_url().contains(env::SYNOPKG_WEB_UI_HOME).not() {
                    return Ok(rouille::Response::redirect_307(env::SYNOPKG_WEB_UI_HOME))
                }
                let mut cmd = std::process::Command::new(env::SYNOPKG_CLI_WEB);
                cmd.current_dir(env::SYNOPKG_PKGDEST);
                cmd.envs(&self.envs)
                .env("SERVER_SOFTWARE", "rust")
                .env("SERVER_PROTOCOL", "HTTP/1.1")
                .env("HTTP_HOST", &request.remote_addr().to_string())
                .env("GATEWAY_INTERFACE", "CGI/1.1")
                .env("REQUEST_METHOD", request.method())
                .env("QUERY_STRING", request.raw_query_string())
                .env("REQUEST_URI", request.raw_url())
                .env("PATH_INFO", &request.url())
                .env("SCRIPT_NAME", ".")
                .env("SCRIPT_FILENAME", &request.url())
                .env("SERVER_PORT", self.port.to_string())
                .env("REMOTE_ADDR", request.remote_addr().to_string())
                .env("SERVER_NAME", request.remote_addr().to_string())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .stdin(Stdio::piped());

                for ele in request.headers() {
                    let k = ele.0.to_uppercase();
                    let v = ele.1;
                    if k == "PROXY" {
                        continue
                    }
                    if v.is_empty().not() {
                        cmd.env(format!("HTTP_{}", k), v);
                    }
                }

                if request.header("Content-Type").unwrap_or_default().is_empty().not() {
                    cmd.env(
                        "CONTENT_TYPE",
                        request.header("Content-Type").context("[XunleiPanelServer] Failed to set Content-Type header")?,
                    );
                }

                if request.header("content-type").unwrap_or_default().is_empty().not() {
                    cmd.env(
                        "CONTENT_TYPE",
                        request.header("content-type").context("[XunleiPanelServer] Failed to set content-type header")?,
                    );
                }

                if request.header("Content-Length").unwrap_or_default().is_empty().not() {
                    cmd.env(
                        "CONTENT_LENGTH",
                        request.header("Content-Length").context("[XunleiPanelServer] Failed to set Content-Length header")?,
                    );
                }

                let mut child = cmd.spawn()?;

                if let Some(mut body) = request.data() {
                    std::io::copy(&mut body, child.stdin.take().as_mut().context("[XunleiPanelServer] Failed to read CGI stdin")?)?;
                }

                {
                    let mut stdout = std::io::BufReader::new(child.stdout.take().context("[XunleiPanelServer] Failed to reader CGI stdout")?);

                    let mut headers = Vec::new();
                    let mut status_code = 200;
                    for header_res in std::io::BufRead::lines(stdout.by_ref()) {
                        let header = header_res?;
                        if header.is_empty() {
                            break;
                        }

                        let (header, val) = header.split_once(':').context("[XunleiPanelServer] Failed to split_once header")?;
                        let val = &val[1..];

                        if header == "Status" {
                            status_code = val[0..3]
                                .parse()
                                .expect("Status returned by CGI program is invalid");
                        } else {
                            headers.push((header.to_owned().into(), val.to_owned().into()));
                        }
                    }
                    Ok(rouille::Response{status_code,headers,data:rouille::ResponseBody::from_reader(stdout),upgrade:None,})
                }
            }
        )
    }
}

impl Running for XunleiPanelServer {
    fn run(self) -> anyhow::Result<()> {
        let sessions_storage: Mutex<HashMap<String, Session>> = Mutex::new(HashMap::new());
        let listen = format!("{}:{}", self.host, self.port);
        log::info!(
            "[XunleiLauncher] Start Xunlei Pannel UI, listening on {}",
            listen
        );
        rouille::start_server(listen, move |request| {
            rouille::log(request, io::stdout(), || {
                rouille::session::session(request, "XUNLEI_SID", 3600, |session| {
                    let mut session_data = if session.client_has_sid() {
                        sessions_storage.lock().unwrap().get(session.id()).cloned()
                    } else {
                        None
                    };

                    let response = self.handle_route(request, &mut session_data);

                    if let Some(d) = session_data {
                        sessions_storage
                            .lock()
                            .unwrap()
                            .insert(session.id().to_owned(), d);
                    } else if session.client_has_sid() {
                        sessions_storage.lock().unwrap().remove(session.id());
                    }

                    match response {
                        Ok(res) => res,
                        Err(e) => Response::text(format!("An error occurred {}", e)),
                    }
                })
            })
        });
    }
}

impl From<(XunleiLauncher, HashMap<String, String>)> for XunleiPanelServer {
    fn from(value: (XunleiLauncher, HashMap<String, String>)) -> Self {
        let launcher = value.0;
        Self {
            auth_user: launcher.auth_user.clone(),
            auth_password: launcher.auth_password.clone(),
            host: launcher.host,
            port: launcher.port,
            envs: value.1,
        }
    }
}
