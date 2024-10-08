//! This module provides functionality to gather system information.
//!
//! # Usage
//!
//! ```
//! use sysinfo_rs::get_machine_info;
//!
//! let machine_info = get_machine_info()?;
//! println!("Machine Info: {:?}", machine_info);
//! ```

pub mod hardware;
pub mod software;

use anyhow::Result;
use hardware::HardwareInfo;
use serde::{Deserialize, Serialize};
use software::SoftwareInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineInfo {
    pub hardware: HardwareInfo,
    pub software: SoftwareInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

impl MachineInfo {
    pub fn builder() -> MachineInfoBuilder {
        MachineInfoBuilder::default()
    }
}

#[derive(Default)]
pub struct MachineInfoBuilder {
    hardware: Option<HardwareInfo>,
    software: Option<SoftwareInfo>,
    extra: Option<serde_json::Value>,
}

impl MachineInfoBuilder {
    pub fn hardware(mut self, hardware: HardwareInfo) -> Self {
        self.hardware = Some(hardware);
        self
    }

    pub fn software(mut self, software: SoftwareInfo) -> Self {
        self.software = Some(software);
        self
    }

    pub fn extra(mut self, extra: serde_json::Value) -> Self {
        self.extra = Some(extra);
        self
    }

    pub fn build(self) -> Result<MachineInfo> {
        Ok(MachineInfo {
            hardware: self
                .hardware
                .ok_or_else(|| anyhow::anyhow!("Hardware info is required"))?,
            software: self
                .software
                .ok_or_else(|| anyhow::anyhow!("Software info is required"))?,
            extra: self.extra,
        })
    }
}

pub fn get_machine_info() -> Result<MachineInfo> {
    MachineInfo::builder()
        .hardware(HardwareInfo::new()?)
        .software(SoftwareInfo::new()?)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system_info::hardware::{BiosInfo, EnclosureInfo, HardwareInfo, SystemInfo};
    use crate::system_info::software::SoftwareInfo;

    #[test]
    fn test_machine_info_builder() -> Result<()> {
        let hardware = HardwareInfo::new()?;
        let software = SoftwareInfo::new()?;

        let machine_info = MachineInfo::builder()
            .hardware(hardware.clone())
            .software(software.clone())
            .extra(serde_json::json!({"custom_field": "value"}))
            .build()?;

        assert_eq!(
            machine_info.hardware.cpu_is_virtual,
            hardware.cpu_is_virtual
        );
        assert_eq!(machine_info.software.os_release, software.os_release);
        assert_eq!(
            machine_info.extra,
            Some(serde_json::json!({"custom_field": "value"}))
        );

        Ok(())
    }

    #[test]
    fn test_get_machine_info() -> Result<()> {
        let machine_info = get_machine_info()?;
        assert!(!machine_info.hardware.mac_addresses.is_empty());
        assert!(!machine_info.software.os_release.is_empty());
        Ok(())
    }

    #[test]
    fn test_machine_info_serialization() -> Result<()> {
        let machine_info = get_machine_info()?;
        let serialized = serde_json::to_string(&machine_info)?;
        let deserialized: MachineInfo = serde_json::from_str(&serialized)?;
        assert_eq!(
            machine_info.hardware.cpu_is_virtual,
            deserialized.hardware.cpu_is_virtual
        );
        assert_eq!(
            machine_info.software.os_release,
            deserialized.software.os_release
        );
        Ok(())
    }

    #[test]
    fn test_forward_compatibility() -> Result<()> {
        let json_data = r#"
        {
          "hardware": {
            "cpu_is_virtual": true,
            "disk_serial_number": "********",
            "mac_addresses": "**:**:**:**:**:**",
            "bios_info": {
              "vendor": "EFI Development Kit II / OVMF",
              "bios_version": "0.0.0",
              "bios_release_date": "02/06/2015",
              "is_virtual_machine": true,
              "system_bios_major_release": "0",
              "system_bios_minor_release": "0"
            },
            "system_info": {
              "manufacturer": "Cloud Provider",
              "product_name": "Cloud ECS",
              "serial_number": "********",
              "uuid": "********-****-****-****-************"
            },
            "enclosure_info": {
              "manufacturer": "Cloud Provider",
              "enclosure_type": "Cloud",
              "version": "pc-i440fx-2.1",
              "serial_number": "",
              "asset_tag_number": ""
            }
          },
          "software": {
            "os_release": "NAME=\"Cloud Linux\"\nVERSION=\"3 (Custom Edition)\"\nID=\"cloudlinux\"\nID_LIKE=\"rhel fedora centos\"\nVERSION_ID=\"3\"\nVARIANT=\"Custom Edition\"\nVARIANT_ID=\"custom\"\nPLATFORM_ID=\"platform:cl8\"\nPRETTY_NAME=\"Cloud Linux 3 (Custom Edition)\"\nANSI_COLOR=\"0;31\"\nHOME_URL=\"https://www.example.com/\"\n",
            "uname": "{\"machine\":\"x86_64\",\"nodename\":\"********\",\"release\":\"6.6.31-cloudlinux\",\"sysname\":\"Linux\",\"version\":\"1 SMP Thu May 23 08:36:57 UTC 2024\"}"
          }
        }"#;

        let deserialized: MachineInfo = serde_json::from_str(json_data)?;

        // Test hardware fields
        assert!(deserialized.hardware.cpu_is_virtual);
        assert_eq!(deserialized.hardware.disk_serial_number, "********");
        assert_eq!(deserialized.hardware.mac_addresses, "**:**:**:**:**:**");

        // Test bios_info fields
        assert_eq!(
            deserialized.hardware.bios_info.vendor,
            "EFI Development Kit II / OVMF"
        );
        assert_eq!(deserialized.hardware.bios_info.bios_version, "0.0.0");
        assert_eq!(
            deserialized.hardware.bios_info.bios_release_date,
            "02/06/2015"
        );
        assert!(deserialized.hardware.bios_info.is_virtual_machine);
        assert_eq!(
            deserialized.hardware.bios_info.system_bios_major_release,
            "0"
        );
        assert_eq!(
            deserialized.hardware.bios_info.system_bios_minor_release,
            "0"
        );

        // Test system_info fields
        assert_eq!(
            deserialized.hardware.system_info.manufacturer,
            "Cloud Provider"
        );
        assert_eq!(deserialized.hardware.system_info.product_name, "Cloud ECS");
        assert_eq!(deserialized.hardware.system_info.serial_number, "********");
        assert_eq!(
            deserialized.hardware.system_info.uuid,
            "********-****-****-****-************"
        );

        // Test enclosure_info fields
        assert_eq!(
            deserialized.hardware.enclosure_info.manufacturer,
            "Cloud Provider"
        );
        assert_eq!(deserialized.hardware.enclosure_info.enclosure_type, "Cloud");
        assert_eq!(
            deserialized.hardware.enclosure_info.version,
            "pc-i440fx-2.1"
        );
        assert_eq!(deserialized.hardware.enclosure_info.serial_number, "");
        assert_eq!(deserialized.hardware.enclosure_info.asset_tag_number, "");

        // Test software fields
        assert!(deserialized.software.os_release.contains("Cloud Linux"));
        assert!(deserialized.software.uname.contains("x86_64"));

        Ok(())
    }

    #[test]
    fn test_backward_compatibility() -> Result<()> {
        let machine_info = MachineInfo {
            hardware: HardwareInfo {
                cpu_is_virtual: true,
                disk_serial_number: "********".to_string(),
                mac_addresses: "**:**:**:**:**:**".to_string(),
                bios_info: BiosInfo {
                    vendor: "Test Vendor".to_string(),
                    bios_version: "1.0".to_string(),
                    bios_release_date: "2023-01-01".to_string(),
                    is_virtual_machine: true,
                    system_bios_major_release: "1".to_string(),
                    system_bios_minor_release: "0".to_string(),
                },
                system_info: SystemInfo {
                    manufacturer: "Test Manufacturer".to_string(),
                    product_name: "Test Product".to_string(),
                    serial_number: "********".to_string(),
                    uuid: "********-****-****-****-************".to_string(),
                },
                enclosure_info: EnclosureInfo {
                    manufacturer: "Test Enclosure".to_string(),
                    enclosure_type: "Test Type".to_string(),
                    version: "1.0".to_string(),
                    serial_number: "********".to_string(),
                    asset_tag_number: "********".to_string(),
                },
                extra: None,
            },
            software: SoftwareInfo {
                os_release: "Test OS 1.0".to_string(),
                uname: "Test Uname".to_string(),
                extra: None,
            },
            extra: None,
        };

        let serialized = serde_json::to_string(&machine_info)?;
        let deserialized: serde_json::Value = serde_json::from_str(&serialized)?;

        // Check that all fields are present in the serialized JSON
        assert!(deserialized["hardware"]["cpu_is_virtual"].is_boolean());
        assert!(deserialized["hardware"]["disk_serial_number"].is_string());
        assert!(deserialized["hardware"]["mac_addresses"].is_string());
        assert!(deserialized["hardware"]["bios_info"]["vendor"].is_string());
        assert!(deserialized["hardware"]["system_info"]["manufacturer"].is_string());
        assert!(deserialized["hardware"]["enclosure_info"]["manufacturer"].is_string());
        assert!(deserialized["software"]["os_release"].is_string());
        assert!(deserialized["software"]["uname"].is_string());

        // Ensure that extra fields are not present
        assert!(deserialized["hardware"]["extra"].is_null());
        assert!(deserialized["software"]["extra"].is_null());
        assert!(deserialized["extra"].is_null());

        Ok(())
    }

    #[test]
    fn test_extra_fields() -> Result<()> {
        let json_data = r#"
        {
          "hardware": {
            "cpu_is_virtual": true,
            "disk_serial_number": "********",
            "mac_addresses": "**:**:**:**:**:**",
            "bios_info": {
              "vendor": "Test Vendor",
              "bios_version": "1.0",
              "bios_release_date": "2023-01-01",
              "is_virtual_machine": true,
              "system_bios_major_release": "1",
              "system_bios_minor_release": "0",
              "extra_bios_field": "extra_value"
            },
            "system_info": {
              "manufacturer": "Test Manufacturer",
              "product_name": "Test Product",
              "serial_number": "********",
              "uuid": "********-****-****-****-************"
            },
            "enclosure_info": {
              "manufacturer": "Test Enclosure",
              "enclosure_type": "Test Type",
              "version": "1.0",
              "serial_number": "********",
              "asset_tag_number": "********"
            },
            "extra_hardware_field": "extra_hardware_value"
          },
          "software": {
            "os_release": "Test OS 1.0",
            "uname": "Test Uname",
            "extra_software_field": "extra_software_value"
          },
          "extra_top_level_field": "extra_top_level_value"
        }"#;

        let deserialized: MachineInfo = serde_json::from_str(json_data)?;

        // Check that known fields are correctly deserialized
        assert!(deserialized.hardware.cpu_is_virtual);
        assert_eq!(deserialized.hardware.disk_serial_number, "********");
        assert_eq!(deserialized.software.os_release, "Test OS 1.0");

        // Check that extra fields are ignored without causing errors
        assert!(deserialized.hardware.extra.is_none());
        assert!(deserialized.software.extra.is_none());
        assert!(deserialized.extra.is_none());

        Ok(())
    }
}
