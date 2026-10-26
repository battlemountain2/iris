#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

use std::rc::Rc;
use adw::prelude::*;
use gtk::gdk::Key;

use crate::ui::command_palette::CommandPalette;
use crate::ui::compose::ComposeDialog;

#[derive(Clone)]
pub struct KeybindingActions {
    pub on_archive: Rc<dyn Fn()>,
    pub on_trash: Rc<dyn Fn()>,
    pub on_reply: Rc<dyn Fn()>,
    pub on_reply_all: Rc<dyn Fn()>,
    pub on_forward: Rc<dyn Fn()>,
    pub on_star: Rc<dyn Fn()>,
    pub on_toggle_read: Rc<dyn Fn()>,
    pub on_focus_search: Rc<dyn Fn()>,
    pub on_back: Rc<dyn Fn()>,
}

pub fn setup_window_keybindings(
    window: &adw::ApplicationWindow,
    palette: Rc<CommandPalette>,
    compose: Rc<ComposeDialog>,
    thread_list_box: gtk::ListBox,
    actions: KeybindingActions,
) {
    let controller = gtk::EventControllerKey::new();

    let win_weak = window.downgrade();
    let actions_clone = actions.clone();
    let palette_clone = palette.clone();
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
            palette_clone.present(&win);
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
            // 'e' -> Archive thread
            Key::e => {
                (actions_clone.on_archive)();
                glib::Propagation::Stop
            }
            // '#' or Delete -> Trash thread
            Key::numbersign | Key::Delete => {
                (actions_clone.on_trash)();
                glib::Propagation::Stop
            }
            // 'r' -> Reply
            Key::r => {
                (actions_clone.on_reply)();
                glib::Propagation::Stop
            }
            // 'a' -> Reply All
            Key::a => {
                (actions_clone.on_reply_all)();
                glib::Propagation::Stop
            }
            // 'f' -> Forward
            Key::f => {
                (actions_clone.on_forward)();
                glib::Propagation::Stop
            }
            // 's' -> Star
            Key::s => {
                (actions_clone.on_star)();
                glib::Propagation::Stop
            }
            // 'u' -> Toggle Read/Unread
            Key::u => {
                (actions_clone.on_toggle_read)();
                glib::Propagation::Stop
            }
            // '/' -> Focus thread search
            Key::slash => {
                (actions_clone.on_focus_search)();
                glib::Propagation::Stop
            }
            // Escape or BackSpace -> Back navigation / deselect
            Key::Escape | Key::BackSpace => {
                (actions_clone.on_back)();
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
