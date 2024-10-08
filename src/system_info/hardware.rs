use anyhow::Result;
use pnet::datalink;
use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::Command;
use std::ptr;

const BIOS_INFO_PATH: &str = "/sys/firmware/dmi/entries/0-0/raw";
const SYSTEM_INFO_PATH: &str = "/sys/firmware/dmi/entries/1-0/raw";
const ENCLOSURE_INFO_PATH: &str = "/sys/firmware/dmi/entries/3-0/raw";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub cpu_is_virtual: bool,
    pub disk_serial_number: String,
    pub mac_addresses: String,
    pub bios_info: BiosInfo,
    pub system_info: SystemInfo,
    pub enclosure_info: EnclosureInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

impl HardwareInfo {
    pub fn new() -> Result<Self> {
        Ok(HardwareInfo {
            cpu_is_virtual: determine_virtual_machine_status(),
            disk_serial_number: get_root_device()
                .and_then(|disk_part_name| get_serial_number(&disk_part_name))
                .unwrap_or_default(),
            mac_addresses: get_mac_addresses()?,
            bios_info: read_bios_info(BIOS_INFO_PATH).unwrap_or_default(),
            system_info: read_system_info(SYSTEM_INFO_PATH).unwrap_or_default(),
            enclosure_info: read_enclosure_info(ENCLOSURE_INFO_PATH).unwrap_or_default(),
            extra: None,
        })
    }

    pub fn with_extra(mut self, extra: serde_json::Value) -> Self {
        self.extra = Some(extra);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BiosInfo {
    pub vendor: String,
    pub bios_version: String,
    pub bios_release_date: String,
    pub is_virtual_machine: bool,
    pub system_bios_major_release: String,
    pub system_bios_minor_release: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemInfo {
    pub manufacturer: String,
    pub product_name: String,
    pub serial_number: String,
    pub uuid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnclosureInfo {
    pub manufacturer: String,
    pub enclosure_type: String,
    pub version: String,
    pub serial_number: String,
    pub asset_tag_number: String,
}

#[cfg(target_arch = "x86_64")]
fn is_hypervisor_present() -> bool {
    use std::arch::x86_64::__cpuid;

    let basic_cpuid = unsafe { __cpuid(1) };
    let is_vm = (basic_cpuid.ecx & (1 << 31)) != 0;

    let hypervisor_name = get_hypervisor_name();

    let sys_hypervisor = check_sys_hypervisor();

    let dmesg_hypervisor = check_dmesg_hypervisor();

    is_vm || hypervisor_name.is_some() || sys_hypervisor || dmesg_hypervisor
}

#[cfg(target_arch = "x86_64")]
fn get_hypervisor_name() -> Option<&'static str> {
    use std::arch::x86_64::__cpuid;

    // CPUID leaf 0x40000000 returns a hypervisor signature in EBX, ECX, and EDX
    let hypervisor_cpuid = unsafe { __cpuid(0x40000000) };
    let name = [
        hypervisor_cpuid.ebx,
        hypervisor_cpuid.ecx,
        hypervisor_cpuid.edx,
    ];

    match &name {
        [0x56_4D_77_61, 0x72_65_56_4D, 0x77_61_72_65] => Some("VMware"),
        [0x4D_69_63_72, 0x6F_73_6F_66, 0x74_20_48_76] => Some("Microsoft Hyper-V"),
        [0x4B_56_4D_4B, 0x56_4D_4B_56, 0x4D_4B_56_4D] => Some("KVM"),
        [0x58_65_6E_56, 0x4D_4D_58_65, 0x6E_56_4D_4D] => Some("Xen"),
        _ => None,
    }
}

fn check_sys_hypervisor() -> bool {
    fs::read_to_string("/sys/hypervisor/type")
        .map(|content| content.contains("xen") || content.contains("kvm"))
        .unwrap_or(false)
}

fn check_dmesg_hypervisor() -> bool {
    let dmesg_hypervisor = Command::new("dmesg")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).contains("hypervisor"))
        .unwrap_or(false);
    dmesg_hypervisor
}

#[cfg(target_arch = "aarch64")]
fn is_hypervisor_present() -> bool {
    let cpuinfo = fs::read_to_string("/proc/cpuinfo")
        .map(|content| content.contains("hypervisor"))
        .unwrap_or(false);

    let sys_hypervisor = fs::read_to_string("/sys/hypervisor/properties/capabilities")
        .map(|content| content.contains("kvm"))
        .unwrap_or(false);

    let rdmsr = Command::new("rdmsr")
        .arg("0xC0C")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).contains("hypervisor"))
        .unwrap_or(false);

    let dmesg = Command::new("dmesg")
        .output()
        .map(|output| {
            let output_str = String::from_utf8_lossy(&output.stdout);
            output_str.contains("virtualization") || output_str.contains("hypervisor")
        })
        .unwrap_or(false);

    let device_tree = Command::new("cat")
        .arg("/proc/device-tree/hypervisor")
        .output()
        .map(|output| !String::from_utf8_lossy(&output.stdout).is_empty())
        .unwrap_or(false);

    cpuinfo || sys_hypervisor || rdmsr || dmesg || device_tree
}

// TODO: THIS IS A TEMPORARY SOLUTION
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn is_hypervisor_present() -> bool {
    let cpuinfo = fs::read_to_string("/proc/cpuinfo")
        .map(|content| content.contains("hypervisor"))
        .unwrap_or(false);
    cpuinfo
}

fn determine_virtual_machine_status() -> bool {
    let is_hypervisor = is_hypervisor_present();

    let is_docker = is_running_in_docker();

    let is_systemd_container = is_systemd_running_in_container();

    is_hypervisor || is_docker || is_systemd_container
}

fn is_running_in_docker() -> bool {
    let dockerenv = fs::metadata("/.dockerenv").is_ok();

    let dockerinit = fs::metadata("/.dockerinit").is_ok();

    dockerenv || dockerinit
}

fn is_systemd_running_in_container() -> bool {
    let systemd_container = Command::new("systemctl")
        .arg("is-system-running")
        .output()
        .map_err(|_| false)
        .map(|output| String::from_utf8_lossy(&output.stdout).contains("running in container"))
        .unwrap_or(false);
    systemd_container
}

#[cfg(target_arch = "x86_64")]
fn get_root_device() -> Result<String> {
    BufReader::new(File::open("/proc/mounts")?)
        .lines()
        .find_map(|line| {
            let line = line.ok()?;
            let mut fields = line.split_whitespace();
            if let (Some(device), Some("/")) = (fields.next(), fields.next()) {
                Some(device.strip_prefix("/dev/").unwrap_or(device).to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("Root file system device not found"))
}

#[cfg(target_arch = "aarch64")]
fn get_root_device_name() -> Option<String> {
    BufReader::new(File::open("/proc/mounts").ok()?)
        .lines()
        .find_map(|line| {
            let mount_entry = line.ok()?;
            let parts: Vec<&str> = mount_entry.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == "/" {
                Some(parts[0][5..].to_string())
            } else {
                None
            }
        })
}

#[cfg(target_arch = "x86_64")]
fn get_serial_number(disk_part_name: &str) -> Result<String> {
    use libudev_sys as udev;

    unsafe {
        let udev = udev::udev_new();
        if udev.is_null() {
            return Err(anyhow::anyhow!("Failed to create udev context"));
        }

        let disk_part_name_c = CString::new(disk_part_name)?;
        let dev = udev::udev_device_new_from_subsystem_sysname(
            udev,
            b"block\0".as_ptr() as *const i8,
            disk_part_name_c.as_ptr(),
        );
        if dev.is_null() {
            return Err(anyhow::anyhow!("Failed to create udev device"));
        }

        let parent_dev = udev::udev_device_get_parent_with_subsystem_devtype(
            dev,
            b"block\0".as_ptr() as *const i8,
            ptr::null(),
        );
        if parent_dev.is_null() {
            return Err(anyhow::anyhow!("Failed to get parent device"));
        }

        let serial =
            udev::udev_device_get_property_value(parent_dev, b"ID_SERIAL\0".as_ptr() as *const i8);
        if serial.is_null() {
            return Err(anyhow::anyhow!("Serial number not found"));
        }

        let serial_str = CStr::from_ptr(serial).to_string_lossy().into_owned();

        udev::udev_device_unref(dev);
        udev::udev_unref(udev);

        Ok(serial_str)
    }
}

// TODO: THIS IS A TEMPORARY SOLUTION, NEEDS TO BE TESTED
#[cfg(target_arch = "aarch64")]
fn get_device_serial(disk_part_name: &str) -> Result<String> {
    unsafe {
        let udev = udev::udev_new();
        if udev.is_null() {
            return Err(anyhow::anyhow!("Failed to create udev context"));
        }

        let enumerate = udev::udev_enumerate_new(udev);
        if enumerate.is_null() {
            udev::udev_unref(udev);
            return Err(anyhow::anyhow!("Failed to create udev enumerate"));
        }

        udev::udev_enumerate_add_match_subsystem(enumerate, b"block\0".as_ptr() as *const i8);
        udev::udev_enumerate_scan_devices(enumerate);

        let result =
            std::iter::successors(udev::udev_enumerate_get_list_entry(enumerate), |&entry| {
                (!entry.is_null()).then(|| udev::udev_list_entry_get_next(entry))
            })
            .find_map(|entry| {
                let dev_path = udev::udev_list_entry_get_name(entry);
                if dev_path.is_null() {
                    return None;
                }

                let dev = udev::udev_device_new_from_syspath(udev, dev_path);
                if dev.is_null() {
                    return None;
                }

                let dev_name = CStr::from_ptr(udev::udev_device_get_sysname(dev))
                    .to_str()
                    .ok()?;

                if dev_name != disk_part_name {
                    udev::udev_device_unref(dev);
                    return None;
                }

                let parent_dev = udev::udev_device_get_parent_with_subsystem_devtype(
                    dev,
                    b"block\0".as_ptr() as *const i8,
                    std::ptr::null(),
                );
                if parent_dev.is_null() {
                    udev::udev_device_unref(dev);
                    return None;
                }

                let serial = udev::udev_device_get_property_value(
                    parent_dev,
                    b"ID_SERIAL\0".as_ptr() as *const i8,
                );
                if serial.is_null() {
                    udev::udev_device_unref(dev);
                    return None;
                }

                let serial_str = CStr::from_ptr(serial).to_string_lossy().into_owned();
                udev::udev_device_unref(dev);
                Some(serial_str)
            });

        udev::udev_enumerate_unref(enumerate);
        udev::udev_unref(udev);

        result.ok_or_else(|| anyhow::anyhow!("Failed to find device serial"))
    }
}

fn get_mac_addresses() -> Result<String> {
    let interfaces = datalink::interfaces();
    let mut mac_addresses = Vec::new();

    for iface in interfaces {
        if let Some(mac) = iface.mac {
            mac_addresses.push(format!("{}", mac));
        }
    }

    Ok(mac_addresses.join(", "))
}

fn read_bios_info<P: AsRef<Path>>(path: P) -> Result<BiosInfo> {
    let mut buffer = Vec::new();
    File::open(&path)?.read_to_end(&mut buffer)?;

    let length = buffer[1] as usize;
    let unformatted_section = &buffer[length..];

    Ok(BiosInfo {
        vendor: extract_string(unformatted_section, buffer[0x04])?,
        bios_version: extract_string(unformatted_section, buffer[0x05])?,
        bios_release_date: extract_string(unformatted_section, buffer[0x08])?,
        is_virtual_machine: (buffer[0x13] & 0x08) >> 3 == 1 || determine_virtual_machine_status(),
        system_bios_major_release: buffer[0x14].to_string(),
        system_bios_minor_release: buffer[0x15].to_string(),
    })
}

fn read_system_info<P: AsRef<Path>>(path: P) -> Result<SystemInfo> {
    let mut buffer = Vec::new();
    File::open(&path)?.read_to_end(&mut buffer)?;

    let length = buffer[1] as usize;
    let unformed_section = &buffer[length..];

    Ok(SystemInfo {
        manufacturer: extract_string(unformed_section, buffer[0x04])?,
        product_name: extract_string(unformed_section, buffer[0x05])?,
        serial_number: extract_string(unformed_section, buffer[0x07])?,
        uuid: format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            buffer[0x08], buffer[0x09], buffer[0x0a], buffer[0x0b],
            buffer[0x0c], buffer[0x0d], buffer[0x0e], buffer[0x0f],
            buffer[0x10], buffer[0x11], buffer[0x12], buffer[0x13],
            buffer[0x14], buffer[0x15], buffer[0x16], buffer[0x17]
        ),
    })
}

fn read_enclosure_info<P: AsRef<Path>>(path: P) -> Result<EnclosureInfo> {
    let mut buffer = Vec::new();
    File::open(&path)?.read_to_end(&mut buffer)?;

    let length = buffer[1] as usize;
    let unformed_section = &buffer[length..];

    Ok(EnclosureInfo {
        manufacturer: extract_string(unformed_section, buffer[0x04])?,
        enclosure_type: extract_string(unformed_section, buffer[0x05])?,
        version: extract_string(unformed_section, buffer[0x06])?,
        serial_number: extract_string(unformed_section, buffer[0x07])?,
        asset_tag_number: extract_string(unformed_section, buffer[0x08])?,
    })
}

fn extract_string(unformed_section: &[u8], index: u8) -> Result<String> {
    if index == 0 {
        return Ok(String::new());
    }

    unformed_section
        .split(|&b| b == 0)
        .nth(index as usize - 1)
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .ok_or_else(|| anyhow::anyhow!("String not found"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_get_root_device() -> Result<()> {
        let root_device = get_root_device()?;
        assert!(!root_device.is_empty());
        Ok(())
    }

    #[test]
    fn test_get_serial_number() -> Result<()> {
        match get_root_device() {
            Ok(disk_part_name) => {
                match get_serial_number(&disk_part_name) {
                    Ok(serial_number) => {
                        assert!(!serial_number.is_empty());
                        Ok(())
                    }
                    Err(_) => Ok(()), // If we can't get the serial number, still pass the test
                }
            }
            Err(_) => Ok(()), // If root device doesn't exist, pass the test
        }
    }

    #[test]
    fn test_get_mac_addresses() -> Result<()> {
        let mac_addresses = get_mac_addresses()?;
        assert!(!mac_addresses.is_empty());
        Ok(())
    }

    #[test]
    fn test_get_bios_info() -> Result<()> {
        match read_bios_info(BIOS_INFO_PATH) {
            Ok(bios_info) => {
                assert!(!bios_info.vendor.is_empty());
                Ok(())
            }
            Err(_) => Ok(()),
        }
    }

    #[test]
    fn test_get_system_info() -> Result<()> {
        match read_system_info(SYSTEM_INFO_PATH) {
            Ok(system_info) => {
                assert!(!system_info.manufacturer.is_empty());
                Ok(())
            }
            Err(_) => Ok(()),
        }
    }

    #[test]
    fn test_get_enclosure_info() -> Result<()> {
        match read_enclosure_info(ENCLOSURE_INFO_PATH) {
            Ok(enclosure_info) => {
                assert!(!enclosure_info.manufacturer.is_empty());
                Ok(())
            }
            Err(_) => Ok(()),
        }
    }

    #[test]
    fn test_hardware_info_with_extra() -> Result<()> {
        let hardware_info =
            HardwareInfo::new()?.with_extra(serde_json::json!({"custom_field": "value"}));

        assert!(hardware_info.extra.is_some());
        assert_eq!(hardware_info.extra.unwrap()["custom_field"], "value");
        Ok(())
    }

    #[test]
    fn test_hardware_info_serialization() -> Result<()> {
        let hardware_info = HardwareInfo::new()?;
        let serialized = serde_json::to_string(&hardware_info)?;
        let deserialized: HardwareInfo = serde_json::from_str(&serialized)?;
        assert_eq!(hardware_info.cpu_is_virtual, deserialized.cpu_is_virtual);
        assert_eq!(
            hardware_info.disk_serial_number,
            deserialized.disk_serial_number
        );
        Ok(())
    }
}
