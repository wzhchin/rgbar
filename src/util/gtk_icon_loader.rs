use std::collections::HashMap;
use std::fs::read_to_string;
use std::rc::Rc;
use std::{cell::RefCell, path::PathBuf};

use log::info;

use crate::prelude::*;

use crate::config::{get_config, ParsedConfig};

#[derive(Clone)]
pub struct GtkIconLoader {
    cache: Rc<RefCell<HashMap<String, gtk::gdk_pixbuf::Pixbuf>>>,
}

impl std::fmt::Debug for GtkIconLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GtkIconLoader").finish()
    }
}

#[derive(PartialEq, Clone, Debug)]
#[allow(dead_code)]
pub enum StatusName {
    CPU,
    RAM,
    WIFI,
    Temperature,

    BatteryFull,
    BatteryHigh,
    BatteryMid,
    BatteryLow,
    BatteryEmpty,

    BattetyPowerCharging,
    BatteryPowerNotCharging,
    BatteryPowerDisconnected,
    BatteryPowerUnknown,
    BatteryPowerFull,

    BatteryConservationOn,
    BatteryConservationOff,
    BatteryConservationUnknown,

    Headphone,
    Headset,

    VolumeHigh,
    VolumeMedium,
    VolumeLow,
    VolumeMute,
}

impl GtkIconLoader {
    pub fn new() -> Self {
        GtkIconLoader {
            cache: RefCell::new(HashMap::new()).into(),
        }
    }

    pub fn load_named_pixbuf(&self, name: &str) -> Option<Pixbuf> {
        info!("get icon {}", name);
        if let Some(image) = self.cache.borrow().get(&name.to_lowercase()) {
            return Some(image.clone());
        };

        let config = get_config();
        let config: &Option<ParsedConfig> = config.as_ref();

        let icon = config.as_ref().and_then(|cfg| {
            let alias = cfg
                .icon
                .alias
                .iter()
                .filter_map(|(key, v)| {
                    if v.iter().any(|n| n.to_lowercase() == name.to_lowercase()) {
                        Some(key.as_str())
                    } else {
                        None
                    }
                })
                .nth(0)
                .unwrap_or(name);

            cfg.icon
                .paths
                .iter()
                .filter_map(|e| {
                    let svg_path = PathBuf::new().join(e).join(format!("{}.svg", alias));
                    if svg_path.exists() {
                        read_to_string(svg_path)
                            .ok()
                            .and_then(|e| load_svg_into_pixbuf(&e, 22, 22).ok())
                    } else {
                        None
                    }
                })
                .nth(0)
        });

        let icon = icon.or_else(|| load_from_icon_theme(name));

        if let Some(pbf) = icon {
            self.cache
                .borrow_mut()
                .insert(name.to_lowercase(), pbf.clone());
            Some(pbf)
        } else {
            None
        }
    }
}

fn load_from_icon_theme(name: &str) -> Option<Pixbuf> {
    gtk::IconTheme::default().and_then(|theme| {
        theme
            .load_icon(name, 44, gtk::IconLookupFlags::empty())
            .ok()
            .flatten()
    })
}

fn load_svg_into_pixbuf(
    svg_data: &str,
    width: i32,
    height: i32,
) -> AResult<gtk::gdk_pixbuf::Pixbuf> {
    let stream =
        gtk::gio::MemoryInputStream::from_bytes(&gtk::glib::Bytes::from(svg_data.as_bytes()));
    let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_stream_at_scale(
        &stream,
        256,
        256,
        true,
        None::<&gtk::gio::Cancellable>,
    )?;

    let pixbuf = pixbuf
        .scale_simple(width * 2, height * 2, InterpType::Bilinear)
        .ok_or(aanyhow!("unable to load from svg"))?;

    Ok(pixbuf)
}

#[macro_export]
macro_rules! include_surface {
    ($path:expr, $width:expr, $height:expr) => {{
        let data = include_str!(concat!("../../res/icons/", $path, ".svg"));
        load_svg_into_pixbuf(data, $width, $height).unwrap()
    }};
}

fn load_fixed_status_pixbuf(status_name: StatusName) -> Pixbuf {
    const BASE_SIZE: i32 = 18;

    match status_name {
        StatusName::CPU => include_surface!("cpu", BASE_SIZE, BASE_SIZE),
        StatusName::RAM => include_surface!("memory", BASE_SIZE * 6 / 5, BASE_SIZE * 6 / 5),
        StatusName::WIFI => include_surface!("wifi", BASE_SIZE, BASE_SIZE),
        StatusName::Temperature => include_surface!("temp", BASE_SIZE, BASE_SIZE),
        StatusName::BatteryFull => {
            include_surface!("battery-full", BASE_SIZE * 11 / 7, BASE_SIZE)
        }
        StatusName::BatteryHigh => {
            include_surface!("battery-high", BASE_SIZE * 11 / 7, BASE_SIZE)
        }
        StatusName::BatteryMid => {
            include_surface!("battery-medium", BASE_SIZE * 11 / 7, BASE_SIZE)
        }
        StatusName::BatteryLow => include_surface!("battery-low", BASE_SIZE * 11 / 7, BASE_SIZE),
        StatusName::BatteryEmpty => {
            include_surface!("battery-empty", BASE_SIZE * 11 / 7, BASE_SIZE)
        }
        StatusName::BattetyPowerCharging => {
            include_surface!("battery-charging", BASE_SIZE, BASE_SIZE)
        }
        StatusName::BatteryPowerNotCharging => {
            include_surface!("battery-connected", BASE_SIZE, BASE_SIZE)
        }
        StatusName::BatteryPowerDisconnected => {
            include_surface!("battery-disconnected", BASE_SIZE, BASE_SIZE)
        }
        StatusName::BatteryPowerFull => {
            include_surface!("battery-connected", BASE_SIZE, BASE_SIZE)
        }
        StatusName::BatteryPowerUnknown => {
            include_surface!("battery-disconnected", BASE_SIZE, BASE_SIZE)
        }
        StatusName::BatteryConservationOn => {
            include_surface!("battery-conser", BASE_SIZE, BASE_SIZE)
        }
        StatusName::BatteryConservationOff => {
            include_surface!("battery-not-conser", BASE_SIZE, BASE_SIZE)
        }
        StatusName::BatteryConservationUnknown => {
            include_surface!("battery-not-conser", BASE_SIZE, BASE_SIZE)
        }
        StatusName::Headphone => {
            include_surface!("audio-headphone", BASE_SIZE, BASE_SIZE)
        }
        StatusName::Headset => {
            include_surface!("audio-headset", BASE_SIZE, BASE_SIZE)
        }
        StatusName::VolumeHigh => {
            include_surface!("audio-volume-high", BASE_SIZE, BASE_SIZE)
        }
        StatusName::VolumeMedium => {
            include_surface!("audio-volume-medium", BASE_SIZE, BASE_SIZE)
        }
        StatusName::VolumeLow => {
            include_surface!("audio-volume-low", BASE_SIZE, BASE_SIZE)
        }
        StatusName::VolumeMute => {
            include_surface!("audio-volume-muted", BASE_SIZE, BASE_SIZE)
        }
    }
}

pub fn load_fixed_status_image(icon_name: StatusName) -> gtk::Image {
    gtk::Image::from_surface(
        load_fixed_status_pixbuf(icon_name)
            .create_surface(2, None::<&Window>)
            .as_ref(),
    )
}

pub fn load_fixed_status_surface(status_name: StatusName) -> Option<gtk::cairo::Surface> {
    load_fixed_status_pixbuf(status_name).create_surface(2, None::<&Window>)
}
