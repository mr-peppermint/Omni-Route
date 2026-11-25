use super::model::{ControlCommand, ControlResponse, SystemBalanceStatus, CONTROL_PORT};
use super::store::{load_or_create_token, ServicePaths};
use std::io;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[derive(Clone)]
pub struct SystemBalanceClient {
    paths: ServicePaths,
}

impl Default for SystemBalanceClient {
    fn default() -> Self {
        Self {
            paths: ServicePaths::system(),
        }
    }
}

impl SystemBalanceClient {
    pub async fn status(&self) -> SystemBalanceStatus {
        match self.command(|token| ControlCommand::Status { token }).await {
            Ok(status) => status,
            Err(error) => SystemBalanceStatus::unavailable(error.to_string()),
        }
    }

    pub async fn set_enabled(&self, enabled: bool) -> io::Result<SystemBalanceStatus> {
        self.command(|token| ControlCommand::SetEnabled { token, enabled })
            .await
    }

    pub async fn refresh(&self) -> io::Result<SystemBalanceStatus> {
        self.command(|token| ControlCommand::Refresh { token })
            .await
    }

    pub async fn set_link_enabled(
        &self,
        key: String,
        enabled: bool,
    ) -> io::Result<SystemBalanceStatus> {
        self.command(|token| ControlCommand::SetLinkEnabled {
            token,
            key,
            enabled,
        })
        .await
    }

    pub async fn set_link_metered(
        &self,
        key: String,
        metered: bool,
    ) -> io::Result<SystemBalanceStatus> {
        self.command(|token| ControlCommand::SetLinkMetered {
            token,
            key,
            metered,
        })
        .await
    }

    async fn command<F>(&self, build: F) -> io::Result<SystemBalanceStatus>
    where
        F: FnOnce(String) -> ControlCommand,
    {
        let token = load_or_create_token(&self.paths)?;
        let command = build(token);
        let stream = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            TcpStream::connect(("127.0.0.1", CONTROL_PORT)),
        )
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "System service is unreachable"))??;
        stream.set_nodelay(true)?;
        let (reader, mut writer) = stream.into_split();
        let mut encoded = serde_json::to_vec(&command)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        encoded.push(b'\n');
        writer.write_all(&encoded).await?;
        writer.shutdown().await?;

        let mut line = Vec::new();
        let mut reader = BufReader::new(reader);
        tokio::time::timeout(
            std::time::Duration::from_secs(3),
            reader.read_until(b'\n', &mut line),
        )
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "System service response timed out"))??;
        if line.len() > 65_536 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "risposta troppo grande",
            ));
        }
        let response: ControlResponse = serde_json::from_slice(&line)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if response.ok {
            Ok(response.status)
        } else {
            Err(io::Error::other(
                response.error.unwrap_or_else(|| "comando rifiutato".into()),
            ))
        }
    }
}

/// Avvia il servizio di sistema chiedendo l'elevazione (UAC) e restituisce
/// l'esito reale del comando `sc.exe`.
pub fn start_service_elevated() -> io::Result<()> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
    use windows_sys::Win32::UI::Shell::{
        ShellExecuteExW, SHELLEXECUTEINFOW, SEE_MASK_NOCLOSEPROCESS,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
    let verb = wide("runas");
    let system_root = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows"));
    let sc_path = system_root.join("System32").join("sc.exe");
    let file = wide(&sc_path.to_string_lossy());
    let parameters = wide(&format!("start {}", super::service::SERVICE_NAME));
    let mut execute: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    execute.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    execute.fMask = SEE_MASK_NOCLOSEPROCESS;
    execute.lpVerb = verb.as_ptr();
    execute.lpFile = file.as_ptr();
    execute.lpParameters = parameters.as_ptr();
    execute.nShow = SW_HIDE;

    // SAFETY: all strings are null-terminated UTF-16 and stay alive until the
    // elevated process has exited. `execute` is zero-initialized as required by
    // ShellExecuteExW, and its process handle is closed below.
    if unsafe { ShellExecuteExW(&mut execute) } == 0 {
        return Err(io::Error::last_os_error());
    }
    if execute.hProcess.is_null() {
        return Err(io::Error::other(
            "Windows did not return a handle for the elevated service command",
        ));
    }

    let wait = unsafe { WaitForSingleObject(execute.hProcess, 30_000) };
    if wait != 0 {
        let error = if wait == 0x0000_0102 {
            io::Error::new(io::ErrorKind::TimedOut, "Windows service start command timed out")
        } else {
            io::Error::last_os_error()
        };
        unsafe { CloseHandle(execute.hProcess) };
        return Err(error);
    }

    let mut exit_code = 0u32;
    let got_exit_code = unsafe { GetExitCodeProcess(execute.hProcess, &mut exit_code) };
    unsafe { CloseHandle(execute.hProcess) };
    if got_exit_code == 0 {
        return Err(io::Error::last_os_error());
    }
    if exit_code != 0 {
        let message = match exit_code {
            5 => {
                "Windows denied permission to start the system service. Approve the UAC prompt and try again."
                    .to_owned()
            }
            1053 => {
                "Windows could not start the service in time. Check Event Viewer for the Service Control Manager error."
                    .to_owned()
            }
            1060 => {
                "The Omni-Route system service is not installed. Run or reinstall the Windows installer as administrator."
                    .to_owned()
            }
            _ => format!("Windows service start command failed with exit code {exit_code}"),
        };
        return Err(io::Error::other(format!(
            "{message} (exit code {exit_code})"
        )));
    }
    Ok(())
}
