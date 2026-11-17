// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

use slint::{ComponentHandle, ToSharedString};

use crate::units::systemd::{
    self, BackupSource, BackupSpec, Compression, ContainerSpec, NetworkSpec, PodSpec, TimerSpec,
    VolumeSpec,
};
use crate::{AppWindow, SystemdState};

const TAB_CONTAINER: i32 = 0;
const TAB_VOLUME: i32 = 1;
const TAB_NETWORK: i32 = 2;
const TAB_POD: i32 = 3;
const TAB_TIMER: i32 = 4;
const TAB_BACKUP: i32 = 5;

pub fn register(app: &AppWindow) {
    let handle = app.as_weak();
    app.global::<SystemdState>().on_generate(move || {
        let ui = handle.unwrap();
        generate(&ui.global::<SystemdState>());
    });

    let handle = app.as_weak();
    app.global::<SystemdState>().on_copy_output(move || {
        let ui = handle.unwrap();
        let state = ui.global::<SystemdState>();
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(displayed(&state).1.as_str());
        }
    });

    let handle = app.as_weak();
    app.global::<SystemdState>().on_save_output(move || {
        let ui = handle.unwrap();
        let state = ui.global::<SystemdState>();
        let (file_name, content) = displayed(&state);
        if content.is_empty() {
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(file_name.as_str())
            .save_file()
        else {
            return;
        };
        match std::fs::write(&path, content.as_str()) {
            Ok(()) => set_status(&state, false, format!("Saved {}", path.display())),
            Err(e) => set_status(&state, true, format!("{}: {e}", path.display())),
        }
    });

    let handle = app.as_weak();
    app.global::<SystemdState>().on_install_output(move || {
        let ui = handle.unwrap();
        let state = ui.global::<SystemdState>();
        if state.get_output().is_empty() {
            return;
        }
        let Some(dir) = target_dir(state.get_tab()) else {
            set_status(
                &state,
                true,
                "Cannot determine the config directory".to_string(),
            );
            return;
        };
        let mut units = vec![(
            state.get_file_name().to_string(),
            state.get_output().to_string(),
        )];
        if !state.get_secondary_output().is_empty() {
            units.push((
                state.get_secondary_file_name().to_string(),
                state.get_secondary_output().to_string(),
            ));
        }
        let borrowed: Vec<(&str, &str)> = units
            .iter()
            .map(|(name, content)| (name.as_str(), content.as_str()))
            .collect();

        let mut result = systemd::install_units(&dir, &borrowed);
        let enable = match state.get_tab() {
            TAB_TIMER => state.get_timer_enable_now(),
            TAB_BACKUP => state.get_backup_enable_now(),
            _ => false,
        };
        if result.is_ok() && enable {
            let timer = state.get_file_name().to_string();
            result = systemd::systemctl_user(&["enable", "--now", &timer])
                .map(|()| {
                    format!(
                        "{}; enabled and started {timer}",
                        result.unwrap_or_default()
                    )
                })
                .map_err(|e| format!("Installed, but enabling {timer} failed: {e}"));
        }
        match result {
            Ok(message) => set_status(&state, false, message),
            Err(e) => set_status(&state, true, e),
        }
    });

    generate(&app.global::<SystemdState>());
}

/// The file name and text of what the output panel currently shows.
fn displayed(state: &SystemdState) -> (String, String) {
    if state.get_show_secondary() && !state.get_secondary_output().is_empty() {
        (
            state.get_secondary_file_name().to_string(),
            state.get_secondary_output().to_string(),
        )
    } else {
        (
            state.get_file_name().to_string(),
            state.get_output().to_string(),
        )
    }
}

fn target_dir(tab: i32) -> Option<std::path::PathBuf> {
    match tab {
        TAB_CONTAINER | TAB_VOLUME | TAB_NETWORK | TAB_POD => systemd::quadlet_dir(),
        _ => systemd::user_unit_dir(),
    }
}

fn set_status(state: &SystemdState, is_error: bool, message: String) {
    state.set_status_is_error(is_error);
    state.set_status(message.to_shared_string());
}

fn generate(state: &SystemdState) {
    state.set_status("".to_shared_string());
    state.set_secondary_output("".to_shared_string());
    state.set_secondary_file_name("".to_shared_string());
    let result = match state.get_tab() {
        TAB_VOLUME => {
            let spec = VolumeSpec {
                name: state.get_volume_name().to_string(),
                driver: state.get_volume_driver().to_string(),
                labels: state.get_volume_labels().to_string(),
            };
            spec.render().map(|text| (text, spec.file_name()))
        }
        TAB_NETWORK => {
            let spec = NetworkSpec {
                name: state.get_network_name().to_string(),
                driver: state.get_network_driver().to_string(),
                subnet: state.get_network_subnet().to_string(),
                gateway: state.get_network_gateway().to_string(),
                ipv6: state.get_network_ipv6(),
                internal: state.get_network_internal(),
            };
            spec.render().map(|text| (text, spec.file_name()))
        }
        TAB_POD => {
            let spec = PodSpec {
                name: state.get_pod_name().to_string(),
                ports: state.get_pod_ports().to_string(),
                network: state.get_pod_network().to_string(),
                start_on_boot: state.get_pod_start_on_boot(),
            };
            spec.render().map(|text| (text, spec.file_name()))
        }
        TAB_BACKUP => {
            let preset = state.get_backup_schedule_index() as usize;
            let spec = BackupSpec {
                name: state.get_backup_name().to_string(),
                source_kind: if state.get_backup_source_index() == 1 {
                    BackupSource::PodmanVolume
                } else {
                    BackupSource::Directory
                },
                source: state.get_backup_source().to_string(),
                target_directory: state.get_backup_target().to_string(),
                compression: Compression::from_index(state.get_backup_compression_index()),
                keep_days: state.get_backup_keep_days().to_string(),
                schedule: match systemd::SCHEDULE_PRESETS.get(preset) {
                    Some(preset) => preset.to_string(),
                    None => state.get_backup_calendar().to_string(),
                },
                persistent: true,
            };
            match (spec.render_timer(), spec.render_service()) {
                (Ok(timer), Ok(service)) => {
                    state.set_secondary_output(service.to_shared_string());
                    state.set_secondary_file_name(spec.service_file_name().to_shared_string());
                    set_calendar_preview(state, &spec.schedule);
                    Ok((timer, spec.timer_file_name()))
                }
                (Err(e), _) | (_, Err(e)) => Err(e),
            }
        }
        TAB_TIMER => {
            let preset = state.get_timer_schedule_index() as usize;
            let spec = TimerSpec {
                name: state.get_timer_name().to_string(),
                description: state.get_timer_description().to_string(),
                command: state.get_timer_command().to_string(),
                working_directory: state.get_timer_working_directory().to_string(),
                schedule: match systemd::SCHEDULE_PRESETS.get(preset) {
                    Some(preset) => preset.to_string(),
                    None => state.get_timer_calendar().to_string(),
                },
                persistent: state.get_timer_persistent(),
                randomized_delay: state.get_timer_delay().to_string(),
            };
            let timer = spec.render_timer();
            let service = spec.render_service();
            match (timer, service) {
                (Ok(timer), Ok(service)) => {
                    state.set_secondary_output(service.to_shared_string());
                    state.set_secondary_file_name(spec.service_file_name().to_shared_string());
                    set_calendar_preview(state, &spec.schedule);
                    Ok((timer, spec.timer_file_name()))
                }
                (Err(e), _) | (_, Err(e)) => Err(e),
            }
        }
        _ => {
            let spec = ContainerSpec {
                name: state.get_container_name().to_string(),
                description: state.get_container_description().to_string(),
                image: state.get_container_image().to_string(),
                ports: state.get_container_ports().to_string(),
                volumes: state.get_container_volumes().to_string(),
                environment: state.get_container_environment().to_string(),
                network: state.get_container_network().to_string(),
                auto_update: state.get_container_auto_update(),
                restart: ["always", "on-failure", "no"]
                    [state.get_container_restart_index().clamp(0, 2) as usize]
                    .to_string(),
                start_on_boot: state.get_container_start_on_boot(),
            };
            spec.render().map(|text| (text, spec.file_name()))
        }
    };

    match result {
        Ok((text, file_name)) => {
            state.set_output(text.to_shared_string());
            state.set_file_name(file_name.to_shared_string());
            state.set_error("".to_shared_string());
        }
        Err(e) => {
            state.set_output("".to_shared_string());
            state.set_error(e.to_shared_string());
        }
    }
}

/// Let systemd parse the schedule; a valid one shows the next elapse times, an invalid one the parser's message.
fn set_calendar_preview(state: &SystemdState, schedule: &str) {
    match systemd::calendar_preview(schedule) {
        Some(Ok(next)) => set_status(state, false, format!("Next: {next}")),
        Some(Err(e)) => set_status(state, true, e),
        None => state.set_status("".to_shared_string()),
    }
}
