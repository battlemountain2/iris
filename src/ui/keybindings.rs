#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

use std::rc::Rc;
use adw::prelude::*;
use gtk::gdk::Key;

use crate::ui::command_palette::CommandPalette;
use crate::ui::compose::ComposeDialog;

pub fn setup_window_keybindings(
    window: &adw::ApplicationWindow,
    palette: Rc<CommandPalette>,
    compose: Rc<ComposeDialog>,
    thread_list_box: gtk::ListBox,
) {
    let controller = gtk::EventControllerKey::new();

    let win_weak = window.downgrade();
    controller.connect_key_pressed(move |_ctrl, key, _code, modifier| {
        let Some(win) = win_weak.upgrade() else {
            return glib::Propagation::Proceed;
        };

        // Don't intercept single-letter keys if focus is in an Entry or TextView
        let is_text_input = gtk::prelude::GtkWindowExt::focus(&win)
            .map(|w| w.is::<gtk::Entry>() || w.is::<gtk::TextView>() || w.is::<gtk::SearchEntry>())
            .unwrap_or(false);

        // Ctrl+K: Command Palette
        if modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK) && key == Key::k {
            palette.present(&win);
            return glib::Propagation::Stop;
        }

        // If typing in an entry, do not intercept single-key navigation
        if is_text_input {
            return glib::Propagation::Proceed;
        }

        match key {
            // 'c' -> Compose
            Key::c => {
                compose.present(&win);
                glib::Propagation::Stop
            }
            // 'j' or Down -> Next thread
            Key::j | Key::Down => {
                navigate_list(&thread_list_box, 1);
                glib::Propagation::Stop
            }
            // 'k' or Up -> Previous thread
            Key::k | Key::Up => {
                navigate_list(&thread_list_box, -1);
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });

    window.add_controller(controller);
}

fn navigate_list(list_box: &gtk::ListBox, delta: i32) {
    let count = {
        let mut count = 0;
        let mut child = list_box.first_child();
        while let Some(c) = child {
            count += 1;
            child = c.next_sibling();
        }
        count
    };

    if count == 0 {
        return;
    }

    let current_index = list_box.selected_row().map(|r| r.index()).unwrap_or(-1);
    let next_index = if current_index < 0 {
        if delta > 0 { 0 } else { count - 1 }
    } else {
        (current_index + delta).clamp(0, count - 1)
    };

    if let Some(row) = list_box.row_at_index(next_index) {
        list_box.select_row(Some(&row));
        row.grab_focus();
    }
}
