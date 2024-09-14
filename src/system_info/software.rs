use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::ffi::CStr;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareInfo {
    pub os_release: String,
    pub uname: String,
}

impl SoftwareInfo {
    pub fn new() -> Result<Self> {
        Ok(Self {
            os_release: get_os_release()?,
            uname: get_uname()?,
        })
    }
}

fn get_os_release() -> Result<String> {
    fs::read_to_string("/etc/os-release").context("Failed to read /etc/os-release")
}

fn get_uname() -> Result<String> {
    let utsname = unsafe {
        let mut info: libc::utsname = std::mem::zeroed();
        if libc::uname(&mut info) != 0 {
            return Err(anyhow::anyhow!("Failed to get uname information"));
        }
        info
    };

    let to_string = |field: [i8; 65]| {
        unsafe { CStr::from_ptr(field.as_ptr()) }
            .to_str()
            .map(String::from)
            .context("Invalid UTF-8 in uname field")
    };

    let uname_info = serde_json::json!({
        "sysname": to_string(utsname.sysname)?,
        "nodename": to_string(utsname.nodename)?,
        "release": to_string(utsname.release)?,
        "version": to_string(utsname.version)?,
        "machine": to_string(utsname.machine)?
    });

    Ok(uname_info.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_os_release() {
        let os_release = get_os_release().unwrap();
        assert!(!os_release.is_empty());
    }

    #[test]
    fn test_get_uname() {
        let uname = get_uname().unwrap();
        assert!(!uname.is_empty());
    }
}
