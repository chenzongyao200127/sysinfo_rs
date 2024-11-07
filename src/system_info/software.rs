use anyhow::{Context, Result};
use rustix::system::uname;
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

    pub fn with_extra(mut self, extra: serde_json::Value) -> Self {
        self.extra = Some(extra);
        self
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
    let uname = uname();

    let mut fields = Vec::with_capacity(6);

    let convert_field = |field: &[u8]| -> Result<String> {
        Ok(std::str::from_utf8(field)
            .context("Invalid UTF-8")?
            .to_owned())
    };

    fields.push(("sysname", convert_field(uname.sysname().to_bytes())?));
    fields.push(("nodename", convert_field(uname.nodename().to_bytes())?));
    fields.push(("release", convert_field(uname.release().to_bytes())?));
    fields.push(("version", convert_field(uname.version().to_bytes())?));
    fields.push(("machine", convert_field(uname.machine().to_bytes())?));
    fields.push(("domainname", convert_field(uname.domainname().to_bytes())?));

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
        let software_info = SoftwareInfo::new()
            .unwrap()
            .with_extra(serde_json::json!({"custom_field": "value"}));

        assert!(software_info.extra.is_some());
        assert_eq!(
            software_info.extra.as_ref().map(|e| &e["custom_field"]),
            Some(&serde_json::json!("value"))
        );
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
