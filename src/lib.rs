//! A library for retrieving system information.
//!
//! This crate provides functionality to gather hardware and software information
//! about the machine it's running on. It includes details such as CPU, disk,
//! network interfaces, operating system, and more.

pub mod system_info;

pub use system_info::{get_machine_info, MachineInfo};

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_machine_info() -> Result<()> {
        let machine_info = get_machine_info()?;
        println!("{:?}", machine_info);
        Ok(())
    }
}
