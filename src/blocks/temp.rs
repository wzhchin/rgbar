use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

use chin_tools::{aanyhow, AResult};

use crate::prelude::*;
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
}

impl TempMonitorBlock {
    pub fn new() -> Self {
        // Try common thermal zones: x86_pkg_temp for Intel CPU, acpitz for ACPI/fallback
        let temp_path = match_type_dir("x86_pkg_temp")
            .or_else(|_| match_type_dir("acpitz"))
            .or_else(|_| match_type_dir("TZSEL"))
            .map(|mut p| {
                p.push("temp");
                p
            })
            .ok();

        if temp_path.is_none() {
            log::warn!("no thermal zone found for temperature monitoring");
        }

        TempMonitorBlock {
            dualchannel: DualChannel::new(10),
            temp_path,
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

        let label = gtk::Label::builder().hexpand(false).build();
        label.style_context().add_class("temp-monitor-label");
        // Start hidden; only show when temp >= 60°C
        label.set_visible(false);

        let label_for_closure = label.clone();
        let widget = label.upcast::<gtk::Widget>();
        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                if let Ok(msg) = receiver.recv().await {
                    match msg {
                        TempMonitorOut::Temp(temp) => {
                            if temp < 20.0 {
                                label_for_closure.set_visible(false);
                            } else {
                                label_for_closure.set_visible(true);
                                label_for_closure
                                    .set_text(format!("{:.0}°C", temp).as_str());

                                // Reset color classes
                                label_for_closure
                                    .style_context()
                                    .remove_class("temp-monitor-warn");
                                label_for_closure
                                    .style_context()
                                    .remove_class("temp-monitor-critical");

                                if temp >= 80.0 {
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
