use chin_wayland_utils::WLWorkspaceBehaiver;
use chin_wayland_utils::{WLWindow, WLWindowBehaiver, WLWindowId, WLWorkspace, WLWorkspaceId};
use gtk::glib::{gformat, GString};

use std::collections::{HashMap, HashSet};
use std::ops::Deref;

use crate::prelude::*;
use crate::util;

#[derive(Debug, PartialEq)]
pub struct WindowWidget {
    pub window: WLWindow,
    pub widget: gtk::Box,
    pub title: Label,
    pub dirty: bool,
}

impl Deref for WindowWidget {
    type Target = WLWindow;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl WindowWidget {
    pub fn new(window: WLWindow, icon_loader: &GtkIconLoader) -> Self {
        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        let icon = gtk::Image::builder().build();
        icon.style_context().add_class("wmw-icon");

        let event_box = gtk::EventBox::builder().child(&icon).build();

        let title = gtk::Label::builder().build();
        title.style_context().add_class("wmw-title");
        if window.is_focused() {
            title.set_label(window.get_title().unwrap_or("Unknown Title"));
        }

        title.set_single_line_mode(true);
        title.set_ellipsize(EllipsizeMode::End);
        title.set_lines(1);
        title.set_line_wrap(true);
        title.set_line_wrap_mode(WrapMode::Char);

        container.set_widget_name(&window.get_id().to_string());
        container.pack_start(&event_box, false, false, 0);
        container.pack_start(&title, false, false, 0);
        container.show_all();

        if let Some(app_id) = window.get_app_id() {
            if let Some(img) = icon_loader.load_named_pixbuf(app_id) {
                icon.set_from_surface(img.create_surface(2, None::<&Window>).as_ref());
            } else {
                log::warn!("unable to get icon for {}", app_id);
            }
        }

        {
            let window = window.clone();
            event_box.connect_button_release_event(move |_, event| match event.button() {
                1 => {
                    let _ = window.focus();
                    Propagation::Stop
                }
                _ => Propagation::Proceed,
            });
        }

        Self {
            window,
            widget: container,
            title,
            dirty: true,
        }
    }

    pub fn update_data(&mut self, window: WLWindow) -> bool {
        if self.window != window {
            self.window = window;
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn update_view(&mut self) {
        if self.dirty {
            if let Some(title) = self.window.get_title() {
                self.title.set_label(title);
            }
            if self.window.is_focused() {
                self.title.set_label(
                    self.window
                        .get_title()
                        .as_ref()
                        .map_or("Unknown Title", |v| v),
                );
                self.title.show();
                self.widget.style_context().add_class("wmw-focus")
            } else {
                self.title.set_text("");
                self.title.hide();
                self.widget.style_context().remove_class("wmw-focus")
            }
            if self.window.is_floating() {
                self.widget.style_context().add_class("wmw-floating")
            } else {
                self.widget.style_context().remove_class("wmw-floating")
            }

            if self.window.is_urgent() {
                self.widget.style_context().add_class("wmw-urgent")
            } else {
                self.widget.style_context().remove_class("wmw-urgent")
            }
            self.dirty = false;
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct WindowContainer {
    pub workspace_id: WLWorkspaceId,
    pub widget_map: HashMap<GString, WindowWidget>,
    pub holder: gtk::Box,
    focused_id: Option<WLWindowId>,
    icon_loader: GtkIconLoader,
    dirty: bool,
    to_remove: Vec<gtk::Box>,
    output_name: String,
}

impl WindowContainer {
    pub fn new(output_name: String) -> Self {
        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        // https://stackoverflow.com/questions/50120555/gtk-stack-wont-change-visible-child-inside-an-event-callback-function
        // > I do not have Granite installed so I can't reproduce the given example. Does Granite.Widgets.Welcome get shown after instantiation? If not, and I quote, "Note that the child widget has to be visible itself (see show) in order to become the visible child of this.". Try to instantiate it first, call show on it and then add it to the Gtk.Stack. It should work.
        container.show_all();
        Self {
            widget_map: Default::default(),
            holder: container,
            focused_id: None,
            icon_loader: util::gtk_icon_loader::GtkIconLoader::new(),
            dirty: true,
            to_remove: Default::default(),
            output_name,
            workspace_id: 0,
        }
    }

    pub fn on_window_overwrite(&mut self, window: &WLWindow) {
        let id = window.get_id();
        let id: GString = gformat!("{}", id);
        if let Some(win) = self.widget_map.get_mut(&id) {
            self.dirty = self.dirty
                || window
                    .get_workspace_id()
                    .is_some_and(|e| e == self.workspace_id);

            let dirty = win.update_data(window.clone());
            self.dirty = dirty || self.dirty;
        } else {
            let ww = WindowWidget::new(window.clone(), &self.icon_loader);
            self.holder.add(&ww.widget);
            self.widget_map.insert(id, ww);
            self.dirty = true;
        }
    }

    pub fn on_window_delete(&mut self, window: &WLWindowId) {
        if let Some(_) = self.widget_map.remove(&gformat!("{}", window)) {
            self.dirty = true;
        }
    }

    pub fn on_workspace_change(&mut self, workspace: &WLWorkspace) {
        if workspace.is_focused()
            && workspace
                .get_monitor_id()
                .is_some_and(|w| w == &self.output_name)
        {
            self.workspace_id = workspace.id;
            self.dirty = true;
        }
    }

    pub fn update_view(&mut self) {
        let mut active_widgets: Vec<_> = self
            .widget_map
            .values_mut()
            .filter(|ww| {
                ww.window
                    .get_workspace_id()
                    .is_some_and(|id| id == self.workspace_id)
            })
            .collect();

        active_widgets.sort_by_key(|w| w.get_x());

        let current_children = self.holder.children();
        let active_widget_ptrs: HashSet<_> = active_widgets
            .iter()
            .map(|ww| ww.widget.clone().upcast::<Widget>())
            .collect();

        for child in current_children.iter() {
            if !active_widget_ptrs.contains(child) {
                self.holder.remove(child);
            }
        }

        for (index, ww) in active_widgets.into_iter().enumerate() {
            ww.update_view();

            if !current_children.contains(&ww.widget.clone().upcast()) {
                self.holder.pack_start(&ww.widget, false, false, 0);
            }

            self.holder.reorder_child(&ww.widget, index as i32);
        }

        self.holder.show_all();
        self.dirty = false;
    }
}
