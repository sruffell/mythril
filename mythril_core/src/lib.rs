#![cfg_attr(not(std), no_std)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(never_type)]
#![feature(const_fn)]
#![feature(get_mut_unchecked)]
#![feature(fixed_size_array)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(lang_items)]

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate log;

pub mod device;
pub mod error;
pub mod interrupt;
pub mod logger;
pub mod memory;
mod registers;
pub mod vcpu;
pub mod vm;
pub mod vmcs;
mod vmexit;
pub mod vmx;
