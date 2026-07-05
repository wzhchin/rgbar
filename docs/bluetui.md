# bluetui 蓝牙设备电量获取实现分析

## 项目概述

- **仓库**: https://github.com/pythops/bluetui
- **语言**: Rust
- **功能**: Linux 下的蓝牙管理 TUI 工具

## 核心依赖

```toml
bluer = { version = "0.17", features = ["full"] }
libdbus-sys = { version = "0.2", features = ["vendored"] }
```

## 电量获取实现

### 关键代码位置

`src/bluetooth.rs`

### 实现方式

bluetui 使用 `bluer` 库与 BlueZ（Linux 蓝牙协议栈）通过 D-Bus 通信来获取设备电量。

```rust
// Device 结构体定义
#[derive(Debug, Clone)]
pub struct Device {
    device: BTDevice,
    pub addr: Address,
    pub icon: &'static str,
    pub alias: String,
    pub is_paired: bool,
    pub is_favorite: bool,
    pub is_trusted: bool,
    pub is_connected: bool,
    pub battery_percentage: Option<u8>,  // 电量百分比
}

// 获取所有设备时的电量读取
pub async fn get_all_devices(
    adapter: &Adapter,
    favorite_devices: &[Address],
) -> AppResult<(Vec<Device>, Vec<Device>)> {
    // ...
    for addr in connected_devices_addresses {
        let device = adapter.device(addr)?;
        
        // 通过 bluer 库的 API 获取电量
        let battery_percentage = device.battery_percentage().await?;
        
        // ...
    }
}
```

### 电量显示逻辑

`src/app.rs` 中根据电量百分比显示不同的 Nerd Font 图标：

```rust
{
    if let Some(battery_percentage) = d.battery_percentage {
        match battery_percentage {
            n if n >= 90 => format!("{battery_percentage}% 󰥈 "),
            n if (80..90).contains(&n) => format!("{battery_percentage}% 󰥅 "),
            n if (70..80).contains(&n) => format!("{battery_percentage}% 󰥄 "),
            n if (60..70).contains(&n) => format!("{battery_percentage}% 󰥃 "),
            n if (50..60).contains(&n) => format!("{battery_percentage}% 󰥂 "),
            n if (40..50).contains(&n) => format!("{battery_percentage}% 󰥁 "),
            n if (30..40).contains(&n) => format!("{battery_percentage}% 󰥀 "),
            n if (20..30).contains(&n) => format!("{battery_percentage}% 󰤿 "),
            n if (10..20).contains(&n) => format!("{battery_percentage}% 󰤾 "),
            _ => format!("{battery_percentage}% 󰤾 "),
        }
    } else {
        String::new()
    }
}
```

## 技术要点

1. **bluer 库**: Rust 的 BlueZ 绑定库，提供异步 API
   - 文档: https://docs.rs/bluer/
   - 通过 D-Bus 与 BlueZ 通信

2. **BlueZ Battery API**: BlueZ 通过 `org.bluez.Battery1` 接口暴露设备电量
   - 只有支持 GATT Battery Service 的设备才能报告电量
   - 需要设备已连接才能获取电量

3. **刷新机制**: 应用每秒通过 `tick()` 方法刷新设备状态

## 参考

- BlueZ D-Bus Battery API: https://github.com/bluez/bluez/blob/master/doc/battery-api.txt
- bluer crate: https://crates.io/crates/bluer
