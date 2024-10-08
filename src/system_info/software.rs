use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::ffi::CStr;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareInfo {
    pub os_release: String,
    pub uname: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

impl SoftwareInfo {
    pub fn new() -> Result<Self> {
        Ok(Self {
            os_release: get_os_release()?,
            uname: get_uname()?,
            extra: None,
        })
    }

    pub fn with_extra(mut self, extra: serde_json::Value) -> Self {
        self.extra = Some(extra);
        self
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

    let to_string = |field: &[libc::c_char]| {
        unsafe { CStr::from_ptr(field.as_ptr()) }
            .to_str()
            .map(String::from)
            .context("Invalid UTF-8 in uname field")
    };

    let uname_info = serde_json::json!({
        "sysname": to_string(&utsname.sysname)?,
        "nodename": to_string(&utsname.nodename)?,
        "release": to_string(&utsname.release)?,
        "version": to_string(&utsname.version)?,
        "machine": to_string(&utsname.machine)?
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

    #[test]
    fn test_software_info_with_extra() -> Result<()> {
        let software_info =
            SoftwareInfo::new()?.with_extra(serde_json::json!({"custom_field": "value"}));

        assert!(software_info.extra.is_some());
        assert_eq!(software_info.extra.unwrap()["custom_field"], "value");
        Ok(())
    }

    #[test]
    fn test_software_info_serialization() -> Result<()> {
        let software_info = SoftwareInfo::new()?;
        let serialized = serde_json::to_string(&software_info)?;
        let deserialized: SoftwareInfo = serde_json::from_str(&serialized)?;
        assert_eq!(software_info.os_release, deserialized.os_release);
        assert_eq!(software_info.uname, deserialized.uname);
        Ok(())
    }
}
