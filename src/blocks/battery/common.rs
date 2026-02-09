use chin_tools::AResult;

use crate::util::fileutil;

use super::PowerStatus::{Charging, Discharging, NotCharging, Unknown};

use super::{BatteryInfo, PowerStatus};

static POWER_INFO_PATH: &str = "/sys/class/power_supply/BATT/uevent";

pub fn get_battery_info() -> AResult<BatteryInfo> {
    read_event(POWER_INFO_PATH)
}

fn read_event(_path: &str) -> AResult<BatteryInfo> {
    let mut name: String = "".to_string();
    let mut status: PowerStatus = PowerStatus::Unknown;
    let mut present: u8 = 0;
    let mut technology: String = "".to_string();
    let mut cycle_count: u32 = 0;
    let mut voltage_min_design: u32 = 0;
    let mut voltage_now: u32 = 0;
    let mut power_now: u32 = 0;
    let mut energy_full_design: u32 = 0;
    let mut energy_full: u32 = 0;
    let mut energy_now: u32 = 0;
    let mut capacity: u8 = 0;
    let mut capacity_level: String = "".to_string();
    let mut model_name: String = "".to_string();
    let mut manufacturer: String = "".to_string();
    let mut serial_numer: String = "".to_string();

    // File hosts must exist in current path before this produces output
    if let Ok(lines) = fileutil::read_lines(POWER_INFO_PATH) {
        // Consumes the iterator, returns an (Optional) String
        for ip in lines.map_while(Result::ok) {
            let mut kv = ip.split("=");
            let k = kv.next().unwrap().to_string();
            let v = kv.next().unwrap().to_string();
            match k.as_str() {
                "POWER_SUPPLY_NAME" => name = v.to_string(),
                "POWER_SUPPLY_STATUS" => {
                    status = match v.to_lowercase().as_str() {
                        "charging" => Charging,
                        "not charging" => NotCharging,
                        "discharging" => Discharging,
                        "full" => PowerStatus::Full,
                        _ => Unknown,
                    };
                }
                "POWER_SUPPLY_PRESENT" => present = v.parse()?,
                "POWER_SUPPLY_TECHNOLOGY" => technology = v,
                "POWER_SUPPLY_CYCLE_COUNT" => cycle_count = v.parse()?,
                "POWER_SUPPLY_VOLTAGE_MIN_DESIGN" => voltage_min_design = v.parse()?,
                "POWER_SUPPLY_VOLTAGE_NOW" => voltage_now = v.parse()?,
                "POWER_SUPPLY_POWER_NOW" => power_now = v.parse()?,
                "POWER_SUPPLY_ENERGY_FULL_DESIGN" => energy_full_design = v.parse()?,
                "POWER_SUPPLY_ENERGY_FULL" => energy_full = v.parse()?,
                "POWER_SUPPLY_ENERGY_NOW" => energy_now = v.parse()?,
                "POWER_SUPPLY_CAPACITY" => capacity = v.parse()?,
                "POWER_SUPPLY_CAPACITY_LEVEL" => capacity_level = v,
                "POWER_SUPPLY_MODEL_NAME" => model_name = v,
                "POWER_SUPPLY_MANUFACTURER" => manufacturer = v,
                "POWER_SUPPLY_SERIAL_NUMBER" => serial_numer = v,
                _ => (),
            }
        }
    }

    Ok(BatteryInfo {
        name,
        status,
        present,
        technology,
        cycle_count,
        voltage_min_design,
        voltage_now,
        power_now,
        energy_full_design,
        energy_full,
        energy_now,
        capacity,
        capacity_level,
        model_name,
        manufacturer,
        serial_numer,
    })
}
