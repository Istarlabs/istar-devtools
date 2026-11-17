// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{AppWindow, DiskInfo, SystemInfoState, units::byte_unit_converter::ByteUnitConverter};
use slint::{ComponentHandle, Model, ToSharedString, VecModel};
use std::rc::Rc;
use sysinfo::{Disks, System};

fn get_desktop_environment() -> String {
    std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("DESKTOP_SESSION"))
        .unwrap_or_else(|_| "Unknown".to_string())
}

fn get_gpu_name() -> String {
    let output = std::process::Command::new("lspci")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok());

    let Some(output) = output else {
        return "Unknown".to_string();
    };

    output
        .lines()
        .find(|l| {
            l.contains("VGA") || l.contains("3D controller") || l.contains("Display controller")
        })
        .and_then(|l| l.split(':').nth(2))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

pub fn register(app: &AppWindow) {
    let mut sys = System::new_all();
    sys.refresh_all();

    let state = app.global::<SystemInfoState>();

    state.set_os_name(
        System::name()
            .unwrap_or_else(|| "Unknown".to_string())
            .to_shared_string(),
    );
    state.set_os_version(
        System::os_version()
            .unwrap_or_else(|| "Unknown".to_string())
            .to_shared_string(),
    );
    state.set_kernel_version(
        System::kernel_version()
            .unwrap_or_else(|| "Unknown".to_string())
            .to_shared_string(),
    );
    state.set_desktop_environment(get_desktop_environment().to_shared_string());

    let cpu_name = sys
        .cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    let cpu_cores = sys.cpus().len();
    state.set_cpu_name(format!("{} ({} cores)", cpu_name, cpu_cores).to_shared_string());

    state.set_gpu_name(get_gpu_name().to_shared_string());

    let total_ram = sys.total_memory();
    let used_ram = sys.used_memory();
    state.set_ram_total(ByteUnitConverter::new(total_ram).to_shared_string());
    state.set_ram_used(ByteUnitConverter::new(used_ram).to_shared_string());

    let disks = Disks::new_with_refreshed_list();
    let disk_entries: Vec<DiskInfo> = disks
        .iter()
        .map(|d| DiskInfo {
            mount: d.mount_point().to_string_lossy().to_shared_string(),
            device: d.name().to_string_lossy().to_shared_string(),
            total: ByteUnitConverter::new(d.total_space()).to_shared_string(),
            used: ByteUnitConverter::new(d.total_space() - d.available_space()).to_shared_string(),
            free: ByteUnitConverter::new(d.available_space()).to_shared_string(),
        })
        .collect();

    state.set_disks(Rc::new(VecModel::from(disk_entries)).into());

    let ui_handle = app.as_weak();
    app.global::<SystemInfoState>().on_copy_all(move || {
        let ui = ui_handle.unwrap();
        let s = ui.global::<SystemInfoState>();

        let disk_count = s.get_disks().row_count();
        let disks_str: String = (0..disk_count)
            .map(|i| {
                let d = s.get_disks().row_data(i).unwrap();
                format!("  - {} ({}): {} total, {} used, {} free\n", d.mount, d.device, d.total, d.used, d.free)
            })
            .collect();

        let text = format!(
            "OS: {} {}\nKernel: {}\nDesktop: {}\nCPU: {}\nGPU: {}\nRAM: {} used / {} total\nDisks:\n{}",
            s.get_os_name(), s.get_os_version(),
            s.get_kernel_version(),
            s.get_desktop_environment(),
            s.get_cpu_name(),
            s.get_gpu_name(),
            s.get_ram_used(), s.get_ram_total(),
            disks_str,
        );

        let mut clipboard = arboard::Clipboard::new().unwrap();
        if let Err(e) = clipboard.set_text(text) {
            eprintln!("Clipboard error: {e}");
        }
    });
}
