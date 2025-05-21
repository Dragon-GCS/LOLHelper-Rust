use log::{error, info, trace};
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

#[derive(Debug)]
pub struct LcuMeta {
    process_name: String,
    pid: u32,
    port: Option<u16>,
    token: Option<String>,
    pub host_url: Option<String>,
}

impl Default for LcuMeta {
    fn default() -> Self {
        Self {
            process_name: "LeagueClientUx.exe".to_string(),
            pid: 0,
            port: None,
            token: None,
            host_url: None,
        }
    }
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
    pub fn refresh_meta(&mut self) -> Result<(), HelperError> {
        let output = Command::new("wmic")
            .args([
                "process",
                "where",
                &format!("caption='{}'", self.process_name),
                "get",
                "processid",
            ])
            .output()
            .expect("failed to execute wmic");

        self.pid = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.trim().parse::<u32>().ok())
            .next()
            .ok_or("未找到进程PID")
            .map_err(|_| HelperError::ClientNotFound)?;
        trace!("客户端进程ID: {}", self.pid);
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
        trace!("客户端命令行参数: {}", cmdline);

        for arg in cmdline.split_whitespace() {
            let arg = arg.trim_matches('"');
            if let Some(port) = arg.strip_prefix("--app-port=") {
                self.port = Some(port.parse().unwrap_or(0));
                info!("客户端端口: {}", self.port.as_ref().unwrap());
            } else if let Some(token) = arg.strip_prefix("--remoting-auth-token=") {
                let token = token.to_string();
                self.token = Some(token);
                info!("客户端Token: {}", self.token.as_ref().unwrap());
            }
        }

        if self.token.is_none() && self.port.is_none() {
            error!("未找到客户端端口和Token");
            return Err(HelperError::ClientNotFound);
        }

        self.host_url = Some(format!(
            "riot:{}@127.0.0.1:{}",
            self.token.as_ref().unwrap(),
            self.port.as_ref().unwrap()
        ));
        info!("客户端host_url: {}", self.host_url.as_ref().unwrap());
        Ok(())
    }
}

#[test]
fn test_get_process_pid_by_name() {
    let mut meta = LcuMeta::default();
    let res = meta.refresh_meta();
    assert!(res.is_ok());
    assert!(meta.pid != 0);
    assert!(meta.port.is_some());
    assert!(meta.token.is_some());
    println!(
        "PID: {}, Port: {}, Token: {}",
        meta.pid,
        meta.port.as_ref().unwrap(),
        meta.token.as_ref().unwrap()
    );
}
