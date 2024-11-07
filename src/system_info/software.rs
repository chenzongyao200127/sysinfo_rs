use anyhow::{Context, Result};
use nix::sys::utsname::uname;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareInfo {
    pub os_release: String,
    pub uname: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

// Cache uname info since it rarely changes
static UNAME_INFO: OnceLock<String> = OnceLock::new();

impl SoftwareInfo {
    pub fn new() -> Result<Self> {
        Ok(Self {
            os_release: get_os_release()?,
            uname: get_cached_uname()?,
            extra: None,
        })
    }
}

fn get_os_release() -> Result<String> {
    fs::read_to_string("/etc/os-release").context("Failed to read /etc/os-release")
}

fn get_cached_uname() -> Result<String> {
    Ok(UNAME_INFO
        .get_or_init(|| get_uname().expect("Failed to get uname info"))
        .clone())
}

fn get_uname() -> Result<String> {
    let uname = uname().context("Failed to get uname info")?;

    let mut fields = Vec::with_capacity(6);

    fields.push(("sysname", uname.sysname().to_string_lossy().into_owned()));
    fields.push(("nodename", uname.nodename().to_string_lossy().into_owned()));
    fields.push(("release", uname.release().to_string_lossy().into_owned()));
    fields.push(("version", uname.version().to_string_lossy().into_owned()));
    fields.push(("machine", uname.machine().to_string_lossy().into_owned()));
    fields.push((
        "domainname",
        uname.domainname().to_string_lossy().into_owned(),
    ));

    let uname_info = serde_json::Map::from_iter(
        fields
            .into_iter()
            .map(|(k, v)| (k.to_owned(), serde_json::Value::String(v))),
    );

    Ok(serde_json::Value::Object(uname_info).to_string())
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
    fn test_software_info_with_extra() {
        let mut software_info = SoftwareInfo::new().expect("Failed to create SoftwareInfo");
        software_info.extra = Some(serde_json::json!({"custom_field": "value"}));

        assert!(software_info.extra.is_some());
        if let Some(extra) = &software_info.extra {
            assert_eq!(extra["custom_field"], "value");
        }
    }

    #[test]
    fn test_software_info_serialization() {
        let software_info = SoftwareInfo::new().unwrap();
        let serialized = serde_json::to_string(&software_info).unwrap();
        let deserialized: SoftwareInfo = serde_json::from_str(&serialized).unwrap();
        assert_eq!(software_info.os_release, deserialized.os_release);
        assert_eq!(software_info.uname, deserialized.uname);
    }
}
