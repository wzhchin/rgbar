use crate::application::MonitorInfo;
use crate::blocks::manager::BlockManager;
use crate::blocks::Block;
use crate::prelude::*;
use gtk_layer_shell::Edge;
use gtk_layer_shell::LayerShell;

#[derive(Clone, Default)]
pub struct WidgetShareInfo {
    pub plug_name: Option<String>,
}

pub struct RGBWindow {
    window: ApplicationWindow,
    monitor_info: MonitorInfo,
    share_info: WidgetShareInfo,
}

impl RGBWindow {
    pub(crate) fn new(application: &Application, monitor_info: &MonitorInfo) -> AResult<Self> {
        let window = ApplicationWindow::new(application);

        window.init_layer_shell();
        window.set_layer(gtk_layer_shell::Layer::Top);
        window.auto_exclusive_zone_enable();

        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        window.set_anchor(Edge::Bottom, false);
        window.set_namespace("gtk-layer-shell");

        window.set_monitor(&monitor_info.monitor);
        window.set_app_paintable(true);

        let share_info = WidgetShareInfo {
            plug_name: monitor_info.plug_name.clone(),
        };

        let mi = monitor_info.clone();
        window.connect_destroy(move |_| {
            log::info!("window is destroied {:?}", mi);
        });

        Ok(Self {
            window,
            share_info,
            monitor_info: monitor_info.clone(),
        })
    }

    pub(crate) fn inject_widgets(&self, bm: &BlockManager) {
        let share_info = &self.share_info;
        log::info!(
            "create bar window for monitor-{} {:?}",
            self.monitor_info.num,
            share_info.plug_name
        );

        let bar = gtk::Box::new(Orientation::Horizontal, 10);

        bar.style_context().add_class("bar");

        let time = bm.time_block.widget(share_info);
        time.style_context().add_class("block");
        bar.pack_end(&time, false, false, 0);

        if let Some(ref battery_block) = bm.battery_block {
            let battery = battery_block.widget(share_info);
            battery.style_context().add_class("block");
            bar.pack_end(&battery, false, false, 0);
        }

        let volume = bm.vol_block.widget(share_info);
        volume.style_context().add_class("block");
        bar.pack_end(&volume, false, false, 0);

        let cpu = bm.cpu_block.widget(share_info);
        cpu.style_context().add_class("block");
        bar.pack_end(&cpu, false, false, 0);

        let memory = bm.memory_block.widget(share_info);
        memory.style_context().add_class("block");
        bar.pack_end(&memory, false, false, 0);

        let netspeed = bm.net_block.widget(share_info);
        netspeed.style_context().add_class("block");
        bar.pack_end(&netspeed, false, false, 0);

        let temp_monitor = bm.temp_block.widget(share_info);
        temp_monitor.style_context().add_class("block");
        bar.pack_end(&temp_monitor, false, false, 0);

        let wayland = bm.wayland_block.widget(share_info);
        bar.pack_start(&wayland, false, false, 0);

        self.window.add(&bar);
        bar.show_all();

        let window = self.window.clone();
        idle_add_local_once(move || {
            window.show();
        });
    }
}
