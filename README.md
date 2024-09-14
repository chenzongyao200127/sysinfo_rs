# sysinfo_rs

A library for retrieving system information.

## Usage

```rust
use sysinfo_rs::get_machine_info;

fn main() -> anyhow::Result<()> {
    let machine_info = get_machine_info()?;
    println!("{:?}", machine_info);
    Ok(())
}
```

## Structure

### Hardware Information
- `hardware`: Contains information about the hardware.
  - `cpu_is_virtual`: Analyzes the results of cpuid command execution to determine if the system is running in a virtual machine (key field).
  - `disk_serial_number`: The serial number of the hard disk.
  - `mac_addresses`: A list of MAC addresses for all network interfaces in the system.
  - `bios_info`: Contains BIOS information, including manufacturer, version, release date, and whether it's a virtual machine.
  - `system_info`: System manufacturer, product name, serial number, and UUID.
  - `enclosure_info`: Chassis information, including manufacturer, type, version, serial number, and asset tag number.

### Software Information
- `software`: Contains information about the software.
  - `os_release`: Operating system version information.
  - `uname`: System uname information, including machine, nodename, release, sysname, and version fields.