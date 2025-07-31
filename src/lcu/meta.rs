use log::{debug, error, info};
use std::ffi::{OsString, c_void};
use std::os::windows::ffi::OsStringExt;
use std::process::Command;
use std::ptr::null_mut;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

use crate::errors::HelperError;

#[repr(C)]
struct UnicodeString {
    length: u16,
    maximum_length: u16,
    buffer: *const u16,
}

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtQueryInformationProcess(
        ProcessHandle: HANDLE,
        ProcessInformationClass: u32,
        ProcessInformation: *mut c_void,
        ProcessInformationLength: u32,
        ReturnLength: *mut u32,
    ) -> u32;
}

const PROCESS_COMMAND_LINE_INFORMATION: u32 = 60;
const LCU_PROCESS_NAME: &str = "LeagueClientUx.exe";

#[derive(Debug, Default)]
pub struct LcuMeta {
    pid: u32,
    port: u16,
    token: String,
    pub host_url: String,
}

impl LcuMeta {
    /// 调用windows API获取进程的命令行参数，可以不需要管理员权限
    /// 代码参考https://jishuzhan.net/article/1869253091128250370
    /// window api文档https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntqueryinformationprocess
    /// 调用"wmic process where caption='LeagueClientUx.exe' get processid"获取进程id
    /// 由于没有管理员权限，wmic无法获取到命令行参数
    /// 首先调用 OpenProcess 获取进程句柄
    /// 然后第一次调用 NtQueryInformationProcess 获取命令参数的长度
    /// 然后分配一个缓冲区，第二次调用 NtQueryInformationProcess 获取命令参数
    pub fn refresh(&mut self) -> Result<String, HelperError> {
        let output = Command::new("wmic")
            .args([
                "process",
                "where",
                &format!("caption='{}'", LCU_PROCESS_NAME),
                "get",
                "processid",
            ])
            .output()
            .expect("failed to execute wmic");

        let pid = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.trim().parse::<u32>().ok())
            .next()
            .ok_or(HelperError::ClientNotFound)?;

        if pid == self.pid && !self.host_url.is_empty() {
            debug!("客户端进程未变更, PID: {}", pid);
            return Ok(self.host_url.clone());
        }
        self.pid = pid;
        debug!("客户端进程ID: {}", pid);

        let cmdline = unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, self.pid)
                .map_err(|_| HelperError::ClientNotFound)?;

            let mut buf_len: u32 = 0;
            // 获取命令参数的长度
            let status = NtQueryInformationProcess(
                handle,
                PROCESS_COMMAND_LINE_INFORMATION,
                null_mut(),
                0,
                &mut buf_len,
            );
            if buf_len == 0 && status != 0xc0000004 {
                CloseHandle(handle).unwrap_unchecked();
                error!("获取命令行参数长度失败, {buf_len}, {status:x}");
                return Err(HelperError::ClientNotFound);
            }
            let mut buffer = vec![0u8; buf_len as usize];
            let status = NtQueryInformationProcess(
                handle,
                PROCESS_COMMAND_LINE_INFORMATION,
                buffer.as_mut_ptr() as *mut c_void,
                buf_len,
                &mut buf_len,
            );
            if status != 0 {
                CloseHandle(handle).unwrap_unchecked();
                error!("获取命令行参数失败, {status:x}");
                return Err(HelperError::ClientCMDLineFailed);
            }

            // 解析 UNICODE_STRING
            let ustr = &*(buffer.as_ptr() as *const UnicodeString);
            let len = ustr.length as usize / 2;
            let slice = std::slice::from_raw_parts(ustr.buffer, len);
            let os_string = OsString::from_wide(slice);
            CloseHandle(handle).unwrap_unchecked();
            // 这里的命令行参数是utf-16编码的，需要转换成utf-8
            os_string.to_string_lossy().to_string()
        };
        debug!("客户端命令行参数: {}", cmdline);

        for arg in cmdline.split_whitespace() {
            let arg = arg.trim_matches('"');
            if let Some(port) = arg.strip_prefix("--app-port=") {
                self.port = port.parse().unwrap_or(0);
                info!("客户端端口: {}", self.port);
            } else if let Some(token) = arg.strip_prefix("--remoting-auth-token=") {
                self.token = token.to_string();
                info!("客户端Token: {}", self.token);
            }
        }

        if self.token.is_empty() && self.port == 0 {
            error!("未找到客户端端口和Token");
            return Err(HelperError::ClientNotFound);
        }

        self.host_url = format!("riot:{}@127.0.0.1:{}", self.token, self.port);
        info!("客户端host_url: {}", self.host_url);
        Ok(self.host_url.clone())
    }
}

#[test]
fn test_get_process_pid_by_name() {
    let mut meta = LcuMeta::default();
    assert!(meta.refresh().is_ok());
    assert!(meta.pid != 0);
    assert!(meta.port != 0);
    assert!(!meta.token.is_empty());
    println!(
        "PID: {}, Port: {}, Token: {}",
        meta.pid, meta.port, meta.token
    );
}
