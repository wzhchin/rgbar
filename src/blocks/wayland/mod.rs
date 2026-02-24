pub mod window_widget;
pub mod workspace_widget;

use chin_wayland_utils::{WLCompositor, WLCompositorBehavier, WLEvent, WLWindowBehaiver};

use crate::blocks::wayland::window_widget::WindowContainer;
use crate::blocks::wayland::workspace_widget::WorkspaceContainer;
use crate::datahodler::channel::DualChannel;
use crate::window::WidgetShareInfo;

use super::Block;

use crate::prelude::*;
use chin_tools::utils::id_util;

use log::error;

#[derive(Clone)]
pub enum OutEvent {
    WLEvent(WLEvent),
}

#[derive(Clone)]
pub enum InEvent {}

pub struct WaylandBlock {
    dualchannel: DualChannel<OutEvent, InEvent>,
}

impl Block for WaylandBlock {
    type Out = OutEvent;
    type In = InEvent;

    fn run(&mut self) -> AResult<()> {
        let sender = self.dualchannel.get_out_sender();

        std::thread::spawn(move || {
            let mut comp = WLCompositor::new()?;
            chin_wayland_utils::niri::event_stream::handle_event_stream(|event| {
                let events = comp.handle_event(event);
                if let Some(events) = events {
                    for ele in events {
                        sender.send(OutEvent::WLEvent(ele)).unwrap();
                    }
                }
            })
        });

        let in_receiver = self.dualchannel.get_in_receiver();
        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                if let Ok(msg) = in_receiver.recv().await {
                    match msg {}
                }
            }
        });

        Ok(())
    }

    fn widget(&self, share_info: &WidgetShareInfo) -> Widget {
        let output_name = share_info
            .plug_name
            .as_ref()
            .map_or_else(id_util::generate_uuid, |s| s.to_owned());

        let mut workspace_container = WorkspaceContainer::new(output_name.clone()).unwrap();

        let mut window_container = WindowContainer::new(output_name.clone());

        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        holder.style_context().add_class("wm");

        holder.pack_start(&workspace_container.holder, false, false, 0);

        holder.pack_start(&window_container.holder, false, false, 0);

        let mut receiver = self.dualchannel.get_out_receiver().clone();

        MainContext::ref_thread_default().spawn_local(async move {
            loop {
                match receiver.recv().await {
                    Ok(msg) => {
                        match msg {
                            OutEvent::WLEvent(event) => {
                                log::debug!("Receive wm event: {:?}", event);

                                match event {
                                    WLEvent::WorkspaceDelete(id) => {
                                        workspace_container.on_workspace_delete(&id);
                                    }
                                    WLEvent::WorkspaceOverwrite(workspace) => {
                                        workspace_container.on_workspace_overwrite(&workspace);
                                        window_container.on_workspace_change(&workspace);
                                    }
                                    WLEvent::WindowDelete(id) => {
                                        window_container.on_window_delete(&id);
                                    }
                                    WLEvent::WindowOverwrite(window) => {
                                        window_container.on_window_overwrite(&window);
                                    }
                                    WLEvent::MonitorDelete(_) => {}
                                    WLEvent::MonitorOverwrite(_) => {}
                                }
                            }
                        }
                        window_container.update_view();
                        workspace_container.update_view();
                    }
                    Err(err) => {
                        error!("unable to receive message: {}", err)
                    }
                }
            }
        });

        holder.clone().upcast()
    }
}

impl WaylandBlock {
    pub fn new() -> Self {
        Self {
            dualchannel: DualChannel::new(30),
        }
    }
}
