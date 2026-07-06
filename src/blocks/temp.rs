use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

use chin_tools::{aanyhow, AResult};

use crate::config::get_config;
use crate::prelude::*;
use crate::util::gtk_icon_loader;
use crate::window::WidgetShareInfo;

use super::Block;

use glob::glob;

// ── Utility functions ────────────────────────────────────────────

pub fn match_type_dir(type_name: &str) -> AResult<PathBuf> {
    let entries = glob("/sys/class/thermal/thermal_zone*/type")?;
    let mut s = String::new();
    for entry in entries {
        let pathbuf = entry?;
        let file = File::open(&pathbuf)?;
        let mut reader = BufReader::new(file);
        s.clear();
        let _content_size = reader.read_to_string(&mut s);
        if s.trim_end() == type_name {
            if let Some(p) = pathbuf.parent() {
                return Ok(p.to_owned());
            }
        }
    }
    Err(aanyhow!("unable to get dir"))
}

/// Find a hwmon device whose name matches one of the given names
/// (e.g. "k10temp", "coretemp", "k8temp"), and return the path to its temp1_input.
pub fn match_hwmon_temp(names: &[&str]) -> AResult<PathBuf> {
    let entries = glob("/sys/class/hwmon/hwmon*/name")?;
    let mut s = String::new();
    for entry in entries {
        let pathbuf = entry?;
        let file = File::open(&pathbuf)?;
        let mut reader = BufReader::new(file);
        s.clear();
        let _ = reader.read_to_string(&mut s);
        if names.contains(&s.trim_end()) {
            if let Some(parent) = pathbuf.parent() {
                let temp_path = parent.join("temp1_input");
                if temp_path.exists() {
                    return Ok(temp_path);
                }
            }
        }
    }
    Err(aanyhow!("no matching hwmon device"))
}

pub fn read_type_temp(temp_file: &PathBuf) -> AResult<f64> {
    let mut s = String::new();
    let file = File::open(temp_file)?;
    let mut reader = BufReader::new(file);

    let _content_size = reader.read_to_string(&mut s)?;
    let temp = i64::from_str_radix(s.trim_end(), 10)?;

    Ok(temp as f64 / 1000.)
}

// ── Temperature Monitor Block ────────────────────────────────────

#[derive(Clone)]
pub enum TempMonitorOut {
    Temp(f64),
}

#[derive(Clone)]
pub enum TempMonitorIn {}

pub struct TempMonitorBlock {
    dualchannel: DualChannel<TempMonitorOut, TempMonitorIn>,
    temp_path: Option<PathBuf>,
    always_show: bool,
    warn_temp: f64,
    alert_temp: f64,
}

impl TempMonitorBlock {
    pub fn new() -> Self {
        // Try common thermal zones: x86_pkg_temp for Intel CPU, acpitz for ACPI/fallback,
        // then hwmon CPU sensors (k10temp, coretemp, etc.), then any thermal zone
        let temp_path = match_type_dir("x86_pkg_temp")
            .ok()
            .or_else(|| match_type_dir("acpitz").ok())
            .or_else(|| match_type_dir("TZSEL").ok())
            .map(|mut p| {
                p.push("temp");
                p
            })
            .or_else(|| {
                // hwmon CPU temperature sensors
                match_hwmon_temp(&["k10temp", "k8temp", "k8temp-pci", "coretemp"]).ok()
            })
            .or_else(|| {
                // Drop-in fallback: any thermal zone that exists
                let entries: Vec<_> = glob("/sys/class/thermal/thermal_zone*/temp")
                    .ok()?
                    .filter_map(|e| e.ok())
                    .collect();
                entries.get(0).map(|p| p.to_owned())
            });

        if temp_path.is_none() {
            log::warn!("no thermal zone found for temperature monitoring");
        }

        // Read config
        let (always_show, warn_temp, alert_temp) = get_config()
            .as_ref()
            .as_ref()
            .and_then(|pc| pc.config.temp.as_ref())
            .map(|tc| {
                (
                    tc.always_show.unwrap_or(false),
                    tc.warn.unwrap_or(60.0),
                    tc.alert.unwrap_or(80.0),
                )
            })
            .unwrap_or((false, 60.0, 80.0));

        TempMonitorBlock {
            dualchannel: DualChannel::new(10),
            temp_path,
            always_show,
            warn_temp,
            alert_temp,
        }
    }
}

impl Block for TempMonitorBlock {
    type Out = TempMonitorOut;
    type In = TempMonitorIn;

    fn run(&mut self) -> AResult<()> {
        let sender = self.dualchannel.get_out_sender();
        let temp_path = self.temp_path.clone();

        timeout_add_seconds_local(2, move || {
            if let Some(ref path) = temp_path {
                if let Ok(temp) = read_type_temp(path) {
                    sender.send(TempMonitorOut::Temp(temp)).unwrap();
                }
            }
            ControlFlow::Continue
        });

        Ok(())
    }

    fn widget(&self, _: &WidgetShareInfo) -> gtk::Widget {
        let mut receiver = self.dualchannel.get_out_receiver();

        let initial_temp = self
            .temp_path
            .as_ref()
            .and_then(|p| read_type_temp(p).ok());

        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .hexpand(false)
            .build();

        let icon = gtk_icon_loader::load_fixed_status_image(StatusName::Temperature);

        let label = gtk::Label::builder().hexpand(false).build();
        label.style_context().add_class("temp-monitor-label");

        if let Some(temp) = initial_temp {
            label.set_text(format!("{:.0}°C", temp).as_str());
        }

        if !self.always_show {
            if initial_temp.map_or(true, |t| t < self.warn_temp) {
                holder.set_visible(false);
            }
        }

        holder.pack_start(&icon, false, false, 0);
        holder.pack_start(&label, false, false, 0);

        let label_for_closure = label.clone();
        let holder_for_closure = holder.clone();
        let widget = holder.upcast::<gtk::Widget>();

        let always_show = self.always_show;
        let warn_temp = self.warn_temp;
        let alert_temp = self.alert_temp;

        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                if let Ok(msg) = receiver.recv().await {
                    match msg {
                        TempMonitorOut::Temp(temp) => {
                            if temp < warn_temp {
                                if always_show {
                                    label_for_closure
                                        .set_text(format!("{:.0}°C", temp).as_str());
                                    label_for_closure
                                        .style_context()
                                        .remove_class("temp-monitor-warn");
                                    label_for_closure
                                        .style_context()
                                        .remove_class("temp-monitor-critical");
                                } else {
                                    holder_for_closure.set_visible(false);
                                }
                            } else {
                                if !always_show {
                                    holder_for_closure.set_visible(true);
                                }
                                label_for_closure
                                    .set_text(format!("{:.0}°C", temp).as_str());

                                // Reset color classes
                                label_for_closure
                                    .style_context()
                                    .remove_class("temp-monitor-warn");
                                label_for_closure
                                    .style_context()
                                    .remove_class("temp-monitor-critical");

                                if temp >= alert_temp {
                                    label_for_closure
                                        .style_context()
                                        .add_class("temp-monitor-critical");
                                } else {
                                    label_for_closure
                                        .style_context()
                                        .add_class("temp-monitor-warn");
                                }
                            }
                        }
                    }
                }
            }
        });

        widget
    }
}
