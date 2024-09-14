//! This module provides functionality to gather system information.
//!
//! # Usage
//!
//! ```
//! use sysinfo_rs::get_machine_info;
//!
//! fn main() -> anyhow::Result<()> {
//!     let machine_info = get_machine_info()?;
//!     println!("Machine Info: {:?}", machine_info);
//!     Ok(())
//! }
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
}

pub fn get_machine_info() -> Result<MachineInfo> {
    Ok(MachineInfo {
        hardware: HardwareInfo::new()?,
        software: SoftwareInfo::new()?,
    })
}
