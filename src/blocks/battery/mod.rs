use crate::datahodler::channel::DualChannel;

use crate::util::gtk_icon_loader::load_fixed_status_surface;
use crate::util::timeutil::second_to_human;
use crate::window::WidgetShareInfo;

use self::bluetooth::BluetoothBatteryMonitor;
use self::common::get_battery_info;
#[cfg(feature = "ideapad")]
use self::ideapad::{get_conservation_mode, ConvervationMode};

use super::Block;

use crate::prelude::*;
use batdiff::seconds_now;
use batdiff::BatDiff;
use chin_tools::AResult;
use std::sync::{Arc, Mutex};

mod batdiff;
mod bluetooth;
mod common;
#[cfg(feature = "ideapad")]
mod ideapad;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PowerStatus {
    NotCharging = 1,
    Discharging = 2,
    Charging = 3,
    Full = 4,
    Unknown = 5,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BatteryInfo {
    pub name: String,
    pub status: PowerStatus,
    pub present: u8,
    pub technology: String,
    pub cycle_count: u32,
    pub voltage_min_design: u32,
    pub voltage_now: u32,
    pub power_now: u32,
    pub energy_full_design: u32,
    pub energy_full: u32,
    pub energy_now: u32,
    pub capacity: u8,
    pub capacity_level: String,
    pub model_name: String,
    pub manufacturer: String,
    pub serial_numer: String,
}

impl BatteryInfo {
    pub fn get_percent(&self) -> u8 {
        self.capacity
    }
}

#[derive(Clone)]
pub enum BatteryOut {
    #[cfg(feature = "ideapad")]
    ConvervationMode(ConvervationMode),
    BatteryInfo(BatteryInfo),
    UnknownBatteryInfo,
}

#[derive(Clone)]
pub enum BatteryIn {}

pub struct BatteryBlock {
    dualchannel: DualChannel<BatteryOut, BatteryIn>,
    init_bat_info: BatteryInfo,
    bt_monitor: BluetoothBatteryMonitor,
}

impl BatteryBlock {
    pub fn new() -> AResult<Self> {
        let dualchannel = DualChannel::new(100);
        let init_bat_info = get_battery_info()?;
        let bt_monitor = BluetoothBatteryMonitor::new();

        Ok(Self {
            dualchannel,
            init_bat_info,
            bt_monitor,
        })
    }
}

impl Block for BatteryBlock {
    type Out = BatteryOut;
    type In = BatteryIn;

    fn run(&mut self) -> AResult<()> {
        let sender = self.dualchannel.get_out_sender();
        self.bt_monitor.start();

        timeout_add_seconds(
            2,
            clone!(
                @strong sender =>
                move || {
                    match get_battery_info() {
                        Ok(info) => sender
                            .send(Self::Out::BatteryInfo(info))
                            .expect("send battery info message"),
                        Err(_) => sender
                            .send(Self::Out::UnknownBatteryInfo)
                            .expect("send battery info message"),
                    };

                    #[cfg(feature = "ideapad")]
                    sender
                        .send(BatteryOut::ConvervationMode(get_conservation_mode()))
                        .unwrap();
                    ControlFlow::Continue
                }
            ),
        );

        Ok(())
    }

    fn widget(&self, _share_info: &WidgetShareInfo) -> gtk::Widget {
        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        let percent_icon = gtk::Image::new();
        let power_status_icon = gtk::Image::new();
        let percent_label = gtk::Label::builder().build();
        let remain_time_label = gtk::Label::builder().build();

        percent_icon.style_context().add_class("f-20");
        percent_label.style_context().add_class("battery-label");
        remain_time_label.style_context().add_class("battery-label");

        #[cfg(feature = "ideapad")]
        let convervation_icon =
            gtk_icon_loader::load_fixed_status_image(StatusName::BatteryConservationOff);

        holder.pack_start(&percent_icon, false, false, 0);
        holder.pack_start(&power_status_icon, false, false, 0);
        #[cfg(feature = "ideapad")]
        holder.pack_start(&convervation_icon, false, false, 0);
        holder.pack_start(&percent_label, false, false, 0);
        holder.pack_start(&remain_time_label, false, false, 0);

        #[cfg(feature = "ideapad")]
        let mut cm_status = ConvervationMode::Unknown;

        let mut bat_diff = BatDiff {
            last_power_status: PowerStatus::Unknown,
            last_percent: 0,
            energy_diff: 0,
            time_diff: 0,
            last_record_seconds: seconds_now(),
            last_record_energy: self.init_bat_info.energy_now as usize,
            last_remain_time_notify_sec: 0,
            last_remain_time_label_time: seconds_now(),
        };

        bat_diff.check_percent(&self.init_bat_info, |percent, mapped| {
            percent_label.set_label(&format!("{}%", percent));
            percent_icon.set_from_surface(load_fixed_status_surface(mapped).as_ref());
        });

        bat_diff.check_power_status(&self.init_bat_info, |mapped| {
            power_status_icon.set_from_surface(load_fixed_status_surface(mapped).as_ref());
        });

        let bt_monitor = self.bt_monitor.clone();
        let current_battery_info = Arc::new(Mutex::new(self.init_bat_info.clone()));

        let current_battery_info_clone = current_battery_info.clone();
        let mut receiver = self.dualchannel.get_out_receiver();
        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                if let Ok(msg) = receiver.recv().await {
                    match msg {
                        #[cfg(feature = "ideapad")]
                        BatteryOut::ConvervationMode(cm) => {
                            if cm_status != cm {
                                cm_status = cm;
                                let mapped = match cm_status {
                                    ConvervationMode::Enable => StatusName::BatteryConservationOn,
                                    ConvervationMode::Disable => StatusName::BatteryConservationOff,
                                    ConvervationMode::Unknown => {
                                        StatusName::BatteryConservationUnknown
                                    }
                                };
                                convervation_icon
                                    .set_from_surface(load_fixed_status_surface(mapped).as_ref())
                            }
                        }
                        BatteryOut::BatteryInfo(bi) => {
                            {
                                let mut info = current_battery_info_clone.lock().unwrap();
                                *info = bi.clone();
                            }
                            bat_diff.check_percent(&bi, |percent, mapped| {
                                tracing::info!("set battery");
                                percent_label.set_label(&format!("{}%", percent));
                                percent_icon
                                    .set_from_surface(load_fixed_status_surface(mapped).as_ref());
                            });

                            bat_diff.check_power_status(&bi, |mapped| {
                                power_status_icon
                                    .set_from_surface(load_fixed_status_surface(mapped).as_ref());
                            });

                            bat_diff.check_remain_time(&bi, |mapped| {
                                if let Some(time) = mapped {
                                    remain_time_label
                                        .set_label(&format!("({})", second_to_human(time)));
                                } else {
                                    remain_time_label.set_label("");
                                }
                            });
                        }
                        BatteryOut::UnknownBatteryInfo => {}
                    }
                }
            }
        });

        let bt_monitor_tooltip = bt_monitor.clone();
        let battery_info_tooltip = current_battery_info.clone();
        holder.connect_query_tooltip(move |_widget, _x, _y, _keyboard, tooltip| {
            let info = battery_info_tooltip.lock().unwrap();
            let percent = info.capacity;
            let icon = get_battery_icon(percent);
            let mut text = format!("{}  {}  {}%", icon, info.name, percent);
            drop(info);

            let bt_batteries = bt_monitor_tooltip.get_batteries();
            for bt in &bt_batteries {
                text.push_str(&format!("\n󰂯  {}  {}%", bt.name, bt.percentage));
            }

            tooltip.set_text(Some(&text));
            true
        });
        holder.set_has_tooltip(true);

        holder.upcast()
    }
}

fn get_battery_icon(percent: u8) -> &'static str {
    match percent {
        n if n >= 90 => "󰥈",
        n if n >= 80 => "󰥅",
        n if n >= 70 => "󰥄",
        n if n >= 60 => "󰥃",
        n if n >= 50 => "󰥂",
        n if n >= 40 => "󰥁",
        n if n >= 30 => "󰥀",
        n if n >= 20 => "󰤿",
        n if n >= 10 => "󰤾",
        _ => "󰤾",
    }
}


