#![no_std]
#![no_main]
#![feature(asm)]
#![feature(never_type)]
#![feature(const_fn)]
#![feature(global_asm)]

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate log;

mod allocator;
mod services;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use multiboot2::MemoryArea;
use mythril_core::vm::VmServices;
use mythril_core::*;
use spin::RwLock;

// Temporary helper function to create a vm for a single core
fn default_vm(core: usize, services: &mut impl VmServices) -> Arc<RwLock<vm::VirtualMachine>> {
    let mut config = vm::VirtualMachineConfig::new(vec![core as u8], 1024);

    // FIXME: When `load_image` may return an error, log the error.
    //
    // Map OVMF directly below the 4GB boundary
    config
        .load_image(
            "OVMF.fd".into(),
            memory::GuestPhysAddr::new((4 * 1024 * 1024 * 1024) - (2 * 1024 * 1024)),
        )
        .unwrap_or(());
    config
        .device_map()
        .register_device(device::com::ComDevice::new(core as u64, 0x3F8))
        .unwrap();
    config
        .device_map()
        .register_device(device::com::ComDevice::new(core as u64, 0x402))
        .unwrap(); // The qemu debug port
    config
        .device_map()
        .register_device(device::pci::PciRootComplex::new())
        .unwrap();
    config
        .device_map()
        .register_device(device::pic::Pic8259::new())
        .unwrap();
    config
        .device_map()
        .register_device(device::pit::Pit8254::new())
        .unwrap();
    config
        .device_map()
        .register_device(device::pos::ProgrammableOptionSelect::new())
        .unwrap();
    config
        .device_map()
        .register_device(device::rtc::CmosRtc::new())
        .unwrap();
    config
        .device_map()
        .register_device(device::qemu_fw_cfg::QemuFwCfg::new())
        .unwrap();

    vm::VirtualMachine::new(config, services).expect("Failed to create vm")
}

fn global_alloc_region(info: &multiboot2::BootInformation) -> (u64, u64) {
    let mem_tag = info
        .memory_map_tag()
        .expect("Missing multiboot memory map tag");

    let available = mem_tag
        .memory_areas()
        .map(|area| (area.start_address(), area.end_address()));

    debug!("Modules:");
    let modules = info.module_tags().map(|module| {
        debug!(
            "  0x{:x}-0x{:x}",
            module.start_address(),
            module.end_address()
        );
        (module.start_address() as u64, module.end_address() as u64)
    });

    let sections_tag = info
        .elf_sections_tag()
        .expect("Missing multiboot elf sections tag");

    debug!("Elf sections:");
    let sections = sections_tag.sections().map(|section| {
        debug!(
            "  0x{:x}-0x{:x}",
            section.start_address(),
            section.end_address()
        );
        (section.start_address(), section.end_address())
    });

    // Avoid allocating over the BootInformation structure itself
    let multiboot_info = [(info.start_address() as u64, info.end_address() as u64)];
    debug!(
        "Multiboot Info: 0x{:x}-0x{:x}",
        info.start_address(),
        info.end_address()
    );

    let excluded = modules
        .chain(sections)
        .chain(multiboot_info.iter().copied());

    // TODO: For now, we just use the portion of the largest available
    // region that is above the highest excluded region.
    let max_excluded = excluded
        .max_by(|left, right| left.1.cmp(&right.1))
        .expect("No max excluded region");

    let largest_region = available
        .max_by(|left, right| (left.1 - left.0).cmp(&(right.1 - right.0)))
        .expect("No largest region");

    if largest_region.0 > max_excluded.1 {
        largest_region
    } else if max_excluded.1 > largest_region.0 && max_excluded.1 < largest_region.1 {
        (max_excluded.1, largest_region.1)
    } else {
        panic!("Unable to find suitable global alloc region")
    }
}

#[no_mangle]
pub extern "C" fn kmain(multiboot_info_addr: usize) -> ! {
    // Setup the actual interrupt handlers
    unsafe { interrupt::idt::init() };

    // Setup our (com0) logger
    log::set_logger(&logger::DirectLogger {})
        .map(|()| log::set_max_level(log::LevelFilter::Info))
        .expect("Failed to set logger");

    let multiboot_info = unsafe { multiboot2::load(multiboot_info_addr) };

    let alloc_region = global_alloc_region(&multiboot_info);

    info!(
        "Allocating from 0x{:x}-{:x}",
        alloc_region.0, alloc_region.1
    );

    unsafe { allocator::Allocator::allocate_from(alloc_region.0, alloc_region.1) }

    let mut multiboot_services = services::Multiboot2Services::new(multiboot_info);
    let mut map = BTreeMap::new();
    map.insert(0usize, default_vm(0, &mut multiboot_services));
    let map: &'static _ = Box::leak(Box::new(map));

    vcpu::smp_entry_point(map)
}
