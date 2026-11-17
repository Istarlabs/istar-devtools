// SPDX-FileCopyrightText: 2026 Pascal Klose
// SPDX-License-Identifier: GPL-3.0-or-later

//! Generators for systemd and Podman Quadlet unit files.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Non-empty, trimmed lines of a multi-line text field.
pub fn lines(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
}

/// A unit name may only contain characters that are safe in a file name and a unit name.
pub fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '@'))
}

#[derive(Debug, Clone, Default)]
pub struct ContainerSpec {
    pub name: String,
    pub description: String,
    pub image: String,
    pub ports: String,
    pub volumes: String,
    pub environment: String,
    pub network: String,
    pub auto_update: bool,
    pub restart: String,
    pub start_on_boot: bool,
}

impl ContainerSpec {
    pub fn file_name(&self) -> String {
        format!("{}.container", self.name.trim())
    }

    pub fn render(&self) -> Result<String, String> {
        let name = self.name.trim();
        if !is_valid_name(name) {
            return Err("Name may only contain letters, digits and - _ . @".to_string());
        }
        let image = self.image.trim();
        if image.is_empty() {
            return Err("Image is required".to_string());
        }

        let mut out = String::new();

        out.push_str("[Unit]\n");
        let description = self.description.trim();
        out.push_str(&format!(
            "Description={}\n",
            if description.is_empty() {
                name
            } else {
                description
            }
        ));

        out.push_str("\n[Container]\n");
        out.push_str(&format!("Image={image}\n"));
        out.push_str(&format!("ContainerName={name}\n"));
        for port in lines(&self.ports) {
            out.push_str(&format!("PublishPort={port}\n"));
        }
        for volume in lines(&self.volumes) {
            out.push_str(&format!("Volume={volume}\n"));
        }
        for variable in lines(&self.environment) {
            out.push_str(&format!("Environment={variable}\n"));
        }
        let network = self.network.trim();
        if !network.is_empty() {
            out.push_str(&format!("Network={network}\n"));
        }
        if self.auto_update {
            out.push_str("AutoUpdate=registry\n");
        }

        let restart = self.restart.trim();
        if !restart.is_empty() && restart != "no" {
            out.push_str(&format!("\n[Service]\nRestart={restart}\n"));
        }

        if self.start_on_boot {
            out.push_str("\n[Install]\nWantedBy=default.target\n");
        }

        Ok(out)
    }
}

#[derive(Debug, Clone, Default)]
pub struct VolumeSpec {
    pub name: String,
    pub driver: String,
    pub labels: String,
}

impl VolumeSpec {
    pub fn file_name(&self) -> String {
        format!("{}.volume", self.name.trim())
    }

    pub fn render(&self) -> Result<String, String> {
        let name = checked_name(&self.name)?;
        let mut out = format!("[Volume]\nVolumeName={name}\n");
        let driver = self.driver.trim();
        if !driver.is_empty() {
            out.push_str(&format!("Driver={driver}\n"));
        }
        for label in lines(&self.labels) {
            out.push_str(&format!("Label={label}\n"));
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, Default)]
pub struct NetworkSpec {
    pub name: String,
    pub driver: String,
    pub subnet: String,
    pub gateway: String,
    pub ipv6: bool,
    pub internal: bool,
}

impl NetworkSpec {
    pub fn file_name(&self) -> String {
        format!("{}.network", self.name.trim())
    }

    pub fn render(&self) -> Result<String, String> {
        let name = checked_name(&self.name)?;
        let subnet = self.subnet.trim();
        let gateway = self.gateway.trim();
        if !gateway.is_empty() && subnet.is_empty() {
            return Err("A gateway needs a subnet".to_string());
        }

        let mut out = format!("[Network]\nNetworkName={name}\n");
        for (key, value) in [
            ("Driver", self.driver.trim()),
            ("Subnet", subnet),
            ("Gateway", gateway),
        ] {
            if !value.is_empty() {
                out.push_str(&format!("{key}={value}\n"));
            }
        }
        if self.ipv6 {
            out.push_str("IPv6=true\n");
        }
        if self.internal {
            out.push_str("Internal=true\n");
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, Default)]
pub struct PodSpec {
    pub name: String,
    pub ports: String,
    pub network: String,
    pub start_on_boot: bool,
}

impl PodSpec {
    pub fn file_name(&self) -> String {
        format!("{}.pod", self.name.trim())
    }

    pub fn render(&self) -> Result<String, String> {
        let name = checked_name(&self.name)?;
        let mut out = format!("[Pod]\nPodName={name}\n");
        for port in lines(&self.ports) {
            out.push_str(&format!("PublishPort={port}\n"));
        }
        let network = self.network.trim();
        if !network.is_empty() {
            out.push_str(&format!("Network={network}\n"));
        }
        if self.start_on_boot {
            out.push_str("\n[Install]\nWantedBy=default.target\n");
        }
        Ok(out)
    }
}

fn checked_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if is_valid_name(name) {
        Ok(name)
    } else {
        Err("Name may only contain letters, digits and - _ . @".to_string())
    }
}

/// The `OnCalendar=` keywords offered as presets, in UI order. Index 4 is a custom expression.
pub const SCHEDULE_PRESETS: [&str; 4] = ["hourly", "daily", "weekly", "monthly"];

#[derive(Debug, Clone, Default)]
pub struct TimerSpec {
    pub name: String,
    pub description: String,
    pub command: String,
    pub working_directory: String,
    pub schedule: String,
    pub persistent: bool,
    pub randomized_delay: String,
}

impl TimerSpec {
    pub fn service_file_name(&self) -> String {
        format!("{}.service", self.name.trim())
    }

    pub fn timer_file_name(&self) -> String {
        format!("{}.timer", self.name.trim())
    }

    fn description(&self) -> &str {
        let description = self.description.trim();
        if description.is_empty() {
            self.name.trim()
        } else {
            description
        }
    }

    fn validate(&self) -> Result<(), String> {
        checked_name(&self.name)?;
        let command = self.command.trim();
        if command.is_empty() {
            return Err("Command is required".to_string());
        }
        let program = command.trim_start_matches(['-', '@', ':', '+', '!']);
        if !program.starts_with('/') {
            return Err(
                "The command must start with an absolute path (e.g. /usr/bin/true)".to_string(),
            );
        }
        if self.schedule.trim().is_empty() {
            return Err("Schedule is required".to_string());
        }
        let delay = self.randomized_delay.trim();
        if !delay.is_empty() && delay.parse::<u64>().is_err() {
            return Err("Randomized delay must be a number of seconds".to_string());
        }
        Ok(())
    }

    pub fn render_service(&self) -> Result<String, String> {
        self.validate()?;
        let mut out = format!(
            "[Unit]\nDescription={}\n\n[Service]\nType=oneshot\nExecStart={}\n",
            self.description(),
            self.command.trim()
        );
        let directory = self.working_directory.trim();
        if !directory.is_empty() {
            out.push_str(&format!("WorkingDirectory={directory}\n"));
        }
        Ok(out)
    }

    pub fn render_timer(&self) -> Result<String, String> {
        self.validate()?;
        let mut out = format!(
            "[Unit]\nDescription=Timer for {}\n\n[Timer]\nOnCalendar={}\n",
            self.description(),
            self.schedule.trim()
        );
        if self.persistent {
            out.push_str("Persistent=true\n");
        }
        let delay = self.randomized_delay.trim();
        if !delay.is_empty() && delay != "0" {
            out.push_str(&format!("RandomizedDelaySec={delay}\n"));
        }
        out.push_str("\n[Install]\nWantedBy=timers.target\n");
        Ok(out)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackupSource {
    #[default]
    Directory,
    PodmanVolume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Compression {
    #[default]
    Gzip,
    Zstd,
    Xz,
}

impl Compression {
    pub fn from_index(index: i32) -> Self {
        match index {
            1 => Compression::Zstd,
            2 => Compression::Xz,
            _ => Compression::Gzip,
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Compression::Gzip => "gz",
            Compression::Zstd => "zst",
            Compression::Xz => "xz",
        }
    }

    fn filter(self) -> &'static str {
        match self {
            Compression::Gzip => "gzip -c",
            Compression::Zstd => "zstd -c",
            Compression::Xz => "xz -c",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BackupSpec {
    pub name: String,
    pub source_kind: BackupSource,
    pub source: String,
    pub target_directory: String,
    pub compression: Compression,
    pub keep_days: String,
    pub schedule: String,
    pub persistent: bool,
}

/// Characters that would escape the quoting of the generated shell command.
fn is_shell_safe(text: &str) -> bool {
    !text
        .chars()
        .any(|c| matches!(c, '\'' | '"' | '$' | '`' | '\\' | '\n' | '\r' | '%'))
}

impl BackupSpec {
    fn extension(&self) -> String {
        format!("tar.{}", self.compression.extension())
    }

    /// The shell script the service runs, before it is escaped for `ExecStart=`.
    fn script(&self) -> Result<String, String> {
        let name = checked_name(&self.name)?;
        let source = self.source.trim();
        let target = self.target_directory.trim();
        if source.is_empty() || target.is_empty() {
            return Err("Source and target directory are required".to_string());
        }
        if ![source, target].iter().all(|text| is_shell_safe(text)) {
            return Err(
                "Paths must not contain quotes, $, backticks, backslashes or %".to_string(),
            );
        }
        if !target.starts_with('/') {
            return Err("The target directory must be an absolute path".to_string());
        }
        let keep = self.keep_days.trim();
        let keep_days = if keep.is_empty() {
            None
        } else {
            Some(
                keep.parse::<u32>()
                    .map_err(|_| "Retention must be a number of days".to_string())?,
            )
        };

        let extension = self.extension();
        let mut script = format!(
            "set -euo pipefail; mkdir -p \"{target}\"; f=\"{target}/{}-$(date +%Y%m%d-%H%M%S).{extension}\"; ",
            name
        );
        match self.source_kind {
            BackupSource::Directory => {
                let path = source.trim_end_matches('/');
                let (parent, base) = path
                    .rsplit_once('/')
                    .ok_or_else(|| "The source must be an absolute path".to_string())?;
                if !path.starts_with('/') || base.is_empty() {
                    return Err("The source must be an absolute path to a directory".to_string());
                }
                let parent = if parent.is_empty() { "/" } else { parent };
                script.push_str(&format!("tar -caf \"$f\" -C \"{parent}\" \"{base}\""));
            }
            BackupSource::PodmanVolume => {
                if !is_valid_name(source) {
                    return Err(
                        "Volume name may only contain letters, digits and - _ . @".to_string()
                    );
                }
                script.push_str(&format!(
                    "podman volume export {source} | {} > \"$f\"",
                    self.compression.filter()
                ));
            }
        }
        if let Some(days) = keep_days.filter(|days| *days > 0) {
            script.push_str(&format!(
                "; find \"{target}\" -maxdepth 1 -name \"{name}-*.{extension}\" -mtime +{days} -delete"
            ));
        }
        Ok(script)
    }

    fn timer_spec(&self) -> Result<TimerSpec, String> {
        let script = self.script()?;
        // systemd expands `$` and `%` in ExecStart, so both are doubled; the script is one single-quoted argument.
        let escaped = script.replace('%', "%%").replace('$', "$$");
        Ok(TimerSpec {
            name: self.name.clone(),
            description: format!("Backup of {}", self.source.trim()),
            command: format!("/usr/bin/bash -c '{escaped}'"),
            working_directory: String::new(),
            schedule: self.schedule.clone(),
            persistent: self.persistent,
            randomized_delay: String::new(),
        })
    }

    pub fn service_file_name(&self) -> String {
        format!("{}.service", self.name.trim())
    }

    pub fn timer_file_name(&self) -> String {
        format!("{}.timer", self.name.trim())
    }

    pub fn render_service(&self) -> Result<String, String> {
        self.timer_spec()?.render_service()
    }

    pub fn render_timer(&self) -> Result<String, String> {
        self.timer_spec()?.render_timer()
    }
}

/// `~/.config/containers/systemd`, where Quadlet looks for rootless units.
pub fn quadlet_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("containers").join("systemd"))
}

/// `~/.config/systemd/user`, where plain user units live.
pub fn user_unit_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("systemd").join("user"))
}

/// Write several unit files to `dir`, then reload the user manager once.
pub fn install_units(dir: &Path, units: &[(&str, &str)]) -> Result<String, String> {
    for (file_name, _) in units {
        if file_name.contains('/') || file_name.starts_with('.') {
            return Err("Invalid file name".to_string());
        }
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;

    let mut written = Vec::new();
    for (file_name, content) in units {
        let target = dir.join(file_name);
        let replaced = target.exists();
        std::fs::write(&target, content).map_err(|e| format!("{}: {e}", target.display()))?;
        written.push(format!(
            "{} {}",
            if replaced { "replaced" } else { "installed" },
            target.display()
        ));
    }

    systemctl_user(&["daemon-reload"])
        .map_err(|e| format!("Files written, but daemon-reload failed: {e}"))?;
    Ok(format!("{}; reloaded the user manager", written.join(", ")))
}

/// Run `systemctl --user ARGS`; the error carries stderr.
pub fn systemctl_user(args: &[&str]) -> Result<(), String> {
    let output = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .map_err(|e| format!("systemctl: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Ask systemd itself to parse a calendar expression and return the next elapse times.
/// `None` when `systemd-analyze` is not available, so the caller can skip the preview.
pub fn calendar_preview(expression: &str) -> Option<Result<String, String>> {
    let output = Command::new("systemd-analyze")
        .args(["calendar", "--iterations=3", expression])
        .output()
        .ok()?;
    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        let next: Vec<&str> = text
            .lines()
            .filter_map(|line| line.trim().strip_prefix("Next elapse:"))
            .chain(
                text.lines()
                    .filter_map(|line| line.trim().strip_prefix("Iter. #")),
            )
            .map(str::trim)
            .collect();
        Some(Ok(next.join("\n")))
    } else {
        Some(Err(String::from_utf8_lossy(&output.stderr)
            .trim()
            .to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> ContainerSpec {
        ContainerSpec {
            name: "web".into(),
            description: "Web server".into(),
            image: "docker.io/library/nginx:latest".into(),
            ports: "8080:80\n\n 8443:443 ".into(),
            volumes: "web-data:/usr/share/nginx/html:Z".into(),
            environment: "TZ=Europe/Berlin".into(),
            network: "web.network".into(),
            auto_update: true,
            restart: "always".into(),
            start_on_boot: true,
        }
    }

    #[test]
    fn renders_full_container_unit() {
        let expected = "[Unit]\nDescription=Web server\n\n[Container]\n\
Image=docker.io/library/nginx:latest\nContainerName=web\nPublishPort=8080:80\nPublishPort=8443:443\n\
Volume=web-data:/usr/share/nginx/html:Z\nEnvironment=TZ=Europe/Berlin\nNetwork=web.network\n\
AutoUpdate=registry\n\n[Service]\nRestart=always\n\n[Install]\nWantedBy=default.target\n";
        assert_eq!(spec().render().unwrap(), expected);
        assert_eq!(spec().file_name(), "web.container");
    }

    #[test]
    fn minimal_unit_omits_optional_sections() {
        let unit = ContainerSpec {
            name: "x".into(),
            image: "img".into(),
            restart: "no".into(),
            ..Default::default()
        }
        .render()
        .unwrap();
        assert_eq!(
            unit,
            "[Unit]\nDescription=x\n\n[Container]\nImage=img\nContainerName=x\n"
        );
    }

    #[test]
    fn rejects_bad_input() {
        assert!(
            ContainerSpec {
                image: "i".into(),
                ..Default::default()
            }
            .render()
            .is_err()
        );
        assert!(
            ContainerSpec {
                name: "../evil".into(),
                image: "i".into(),
                ..Default::default()
            }
            .render()
            .is_err()
        );
        assert!(
            ContainerSpec {
                name: "ok".into(),
                ..Default::default()
            }
            .render()
            .is_err()
        );
    }

    #[test]
    fn renders_volume_network_and_pod() {
        let volume = VolumeSpec {
            name: "data".into(),
            driver: "local".into(),
            labels: "app=web\nenv=prod".into(),
        };
        assert_eq!(
            volume.render().unwrap(),
            "[Volume]\nVolumeName=data\nDriver=local\nLabel=app=web\nLabel=env=prod\n"
        );
        assert_eq!(volume.file_name(), "data.volume");

        let network = NetworkSpec {
            name: "web".into(),
            subnet: "10.89.0.0/24".into(),
            gateway: "10.89.0.1".into(),
            ipv6: true,
            internal: true,
            ..Default::default()
        };
        assert_eq!(
            network.render().unwrap(),
            "[Network]\nNetworkName=web\nSubnet=10.89.0.0/24\nGateway=10.89.0.1\nIPv6=true\nInternal=true\n"
        );

        let pod = PodSpec {
            name: "stack".into(),
            ports: "8080:80".into(),
            network: "web.network".into(),
            start_on_boot: true,
        };
        assert_eq!(
            pod.render().unwrap(),
            "[Pod]\nPodName=stack\nPublishPort=8080:80\nNetwork=web.network\n\n[Install]\nWantedBy=default.target\n"
        );
    }

    #[test]
    fn network_gateway_needs_subnet() {
        let network = NetworkSpec {
            name: "n".into(),
            gateway: "10.0.0.1".into(),
            ..Default::default()
        };
        assert!(network.render().is_err());
        assert!(
            VolumeSpec {
                name: "bad name".into(),
                ..Default::default()
            }
            .render()
            .is_err()
        );
    }

    #[test]
    fn renders_timer_and_service() {
        let spec = TimerSpec {
            name: "backup".into(),
            description: "Nightly backup".into(),
            command: "/usr/local/bin/backup.sh".into(),
            working_directory: "/srv".into(),
            schedule: "daily".into(),
            persistent: true,
            randomized_delay: "300".into(),
        };
        assert_eq!(
            spec.render_service().unwrap(),
            "[Unit]\nDescription=Nightly backup\n\n[Service]\nType=oneshot\nExecStart=/usr/local/bin/backup.sh\nWorkingDirectory=/srv\n"
        );
        assert_eq!(
            spec.render_timer().unwrap(),
            "[Unit]\nDescription=Timer for Nightly backup\n\n[Timer]\nOnCalendar=daily\nPersistent=true\nRandomizedDelaySec=300\n\n[Install]\nWantedBy=timers.target\n"
        );
        assert_eq!(spec.timer_file_name(), "backup.timer");
        assert_eq!(spec.service_file_name(), "backup.service");
    }

    #[test]
    fn timer_rejects_relative_command_and_bad_delay() {
        let base = TimerSpec {
            name: "t".into(),
            command: "/bin/true".into(),
            schedule: "daily".into(),
            ..Default::default()
        };
        assert!(base.render_timer().is_ok());
        assert!(
            TimerSpec {
                command: "backup.sh".into(),
                ..base.clone()
            }
            .render_service()
            .is_err()
        );
        assert!(
            TimerSpec {
                randomized_delay: "5min".into(),
                ..base.clone()
            }
            .render_timer()
            .is_err()
        );
        assert!(
            TimerSpec {
                schedule: " ".into(),
                ..base
            }
            .render_timer()
            .is_err()
        );
    }

    fn backup() -> BackupSpec {
        BackupSpec {
            name: "docs".into(),
            source_kind: BackupSource::Directory,
            source: "/home/me/Documents/".into(),
            target_directory: "/srv/backups".into(),
            compression: Compression::Zstd,
            keep_days: "14".into(),
            schedule: "daily".into(),
            persistent: true,
        }
    }

    #[test]
    fn directory_backup_service_escapes_for_systemd() {
        let service = backup().render_service().unwrap();
        assert!(service.contains("Description=Backup of /home/me/Documents/\n"));
        assert!(service.contains(
            "ExecStart=/usr/bin/bash -c 'set -euo pipefail; mkdir -p \"/srv/backups\"; f=\"/srv/backups/docs-$$(date +%%Y%%m%%d-%%H%%M%%S).tar.zst\"; tar -caf \"$$f\" -C \"/home/me\" \"Documents\"; find \"/srv/backups\" -maxdepth 1 -name \"docs-*.tar.zst\" -mtime +14 -delete'\n"
        ));
        assert!(
            backup()
                .render_timer()
                .unwrap()
                .contains("OnCalendar=daily\nPersistent=true\n")
        );
    }

    #[test]
    fn volume_backup_pipes_through_the_compressor() {
        let spec = BackupSpec {
            source_kind: BackupSource::PodmanVolume,
            source: "web-data".into(),
            compression: Compression::Gzip,
            keep_days: "0".into(),
            ..backup()
        };
        let service = spec.render_service().unwrap();
        assert!(service.contains("podman volume export web-data | gzip -c > \"$$f\"'"));
        assert!(!service.contains("find "));
    }

    #[test]
    fn backup_rejects_unsafe_input() {
        assert!(
            BackupSpec {
                source: "/data/it's".into(),
                ..backup()
            }
            .render_service()
            .is_err()
        );
        assert!(
            BackupSpec {
                target_directory: "relative".into(),
                ..backup()
            }
            .render_service()
            .is_err()
        );
        assert!(
            BackupSpec {
                keep_days: "x".into(),
                ..backup()
            }
            .render_service()
            .is_err()
        );
        assert!(
            BackupSpec {
                source: "rel/dir".into(),
                ..backup()
            }
            .render_service()
            .is_err()
        );
        assert!(
            BackupSpec {
                source_kind: BackupSource::PodmanVolume,
                source: "a b".into(),
                ..backup()
            }
            .render_service()
            .is_err()
        );
    }
}
