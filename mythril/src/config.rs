use serde::Deserialize;
use serde::export::Vec;
use alloc::string::String;

#[derive(Deserialize)]
#[derive(Debug)]
pub struct VmConfig {
    pub memory: u64,
    pub kernel: String,
    pub initramfs: String,
    pub cmdline: String,
}

#[derive(Deserialize)]
#[derive(Debug)]
pub struct Config {
    pub version: u64,
    pub vms: Vec<VmConfig>,
}
