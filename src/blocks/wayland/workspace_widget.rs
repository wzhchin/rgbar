use std::{borrow::Cow, collections::HashMap};

use chin_tools::AResult;
use chin_wayland_utils::{WLWorkspace, WLWorkspaceBehaiver, WLWorkspaceId};
pub use gtk::traits::{BoxExt, LabelExt, StyleContextExt, WidgetExt};
use log::error;

#[derive(Debug, PartialEq)]
pub struct WorkspaceWidget {
    workspace: WLWorkspace,
}

impl WorkspaceWidget {
    pub fn new(workspace: WLWorkspace) -> WorkspaceWidget {
        WorkspaceWidget { workspace }
    }
}

#[derive(Debug)]
pub struct WorkspaceContainer {
    pub workspace_widget_map: HashMap<WLWorkspaceId, WorkspaceWidget>,
    pub holder: gtk::Box,
    indicator: gtk::Label,
    output_name: String,
    current_workspace_id: Option<WLWorkspaceId>,
    dirty: bool,
}

impl WorkspaceContainer {
    pub fn new(output_name: String) -> AResult<Self> {
        let holder = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        let indicator = gtk::Label::builder().build();
        indicator.style_context().add_class("ws");

        holder.style_context().add_class("wss");
        holder.pack_start(&indicator, false, true, 0);

        Ok(Self {
            workspace_widget_map: Default::default(),
            holder,
            output_name,
            current_workspace_id: Default::default(),
            indicator,
            dirty: true,
        })
    }

    pub fn on_workspace_overwrite(&mut self, workspace: &WLWorkspace) {
        if workspace
            .get_monitor_id()
            .is_some_and(|n| n == &self.output_name)
        {
            if workspace.is_active() {
                self.current_workspace_id
                    .replace(workspace.get_id().clone());
            }
            self.workspace_widget_map.insert(
                workspace.get_id().to_owned(),
                WorkspaceWidget::new(workspace.clone()),
            );
            self.dirty = true
        }
    }

    pub fn on_workspace_delete(&mut self, id: &WLWorkspaceId) {
        self.workspace_widget_map.remove(id);
    }

    pub fn update_view(&mut self) {
        let indicator = self
            .current_workspace_id
            .and_then(|e| {
                self.workspace_widget_map.get(&e).and_then(|e| {
                    if e.workspace.output.as_ref() == Some(&self.output_name) {
                        Some(e)
                    } else {
                        None
                    }
                })
            })
            .map(|w| w.workspace.get_name())
            .unwrap_or(Cow::Borrowed("?"));
        self.indicator.set_label(
            format!(
                "{} / {}",
                indicator,
                self.workspace_widget_map
                    .iter()
                    .filter(|(_, ws)| ws.workspace.output.as_ref() == Some(&self.output_name))
                    .count()
            )
            .as_str(),
        );
        self.dirty = false;
    }
}
