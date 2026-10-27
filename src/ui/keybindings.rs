#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::Cell;
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
    pub on_sync: Rc<dyn Fn()>,
    pub on_show_shortcuts: Rc<dyn Fn()>,
    pub on_settings: Rc<dyn Fn()>,
    pub on_undo: Rc<dyn Fn()>,
    pub on_nav_folder: Rc<dyn Fn(&str)>,
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
    let pending_g = Rc::new(Cell::new(false));

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

        // Ctrl+R: Global Synchronize Mailboxes
        if modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK) && key == Key::r {
            (actions_clone.on_sync)();
            return glib::Propagation::Stop;
        }

        // Ctrl+,: Preferences & Settings
        if modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK) && key == Key::comma {
            (actions_clone.on_settings)();
            return glib::Propagation::Stop;
        }

        // Ctrl+Z: Undo last triage action
        if modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK) && (key == Key::z || key == Key::Z) {
            (actions_clone.on_undo)();
            return glib::Propagation::Stop;
        }

        // If typing in an entry, do not intercept single-key navigation
        if is_text_input {
            return glib::Propagation::Proceed;
        }

        // '?' or Shift+'/' -> Keyboard Shortcuts Cheatsheet
        if key == Key::question || (modifier.contains(gtk::gdk::ModifierType::SHIFT_MASK) && key == Key::slash) {
            (actions_clone.on_show_shortcuts)();
            return glib::Propagation::Stop;
        }

        // 'g' sequence chords (g i -> Inbox, g s -> Starred, g d -> Drafts, g a -> Archive, g t -> Trash, g e -> Sent, g f -> Feed)
        if pending_g.get() {
            pending_g.set(false);
            match key {
                Key::i => {
                    (actions_clone.on_nav_folder)("inbox");
                    return glib::Propagation::Stop;
                }
                Key::s => {
                    (actions_clone.on_nav_folder)("view:flagged");
                    return glib::Propagation::Stop;
                }
                Key::d => {
                    (actions_clone.on_nav_folder)("folder:drafts");
                    return glib::Propagation::Stop;
                }
                Key::a => {
                    (actions_clone.on_nav_folder)("folder:archive");
                    return glib::Propagation::Stop;
                }
                Key::t => {
                    (actions_clone.on_nav_folder)("folder:trash");
                    return glib::Propagation::Stop;
                }
                Key::e => {
                    (actions_clone.on_nav_folder)("folder:sent");
                    return glib::Propagation::Stop;
                }
                Key::f => {
                    (actions_clone.on_nav_folder)("view:feed");
                    return glib::Propagation::Stop;
                }
                _ => {}
            }
        }

        // 'g' initiates a 2-key chord
        if key == Key::g {
            pending_g.set(true);
            let pg = pending_g.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(1200), move || {
                pg.set(false);
            });
            return glib::Propagation::Stop;
        }

        match key {
            // 'z' -> Undo
            Key::z => {
                (actions_clone.on_undo)();
                glib::Propagation::Stop
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn test_keybinding_actions_invocation() {
        let invoked = Rc::new(AtomicBool::new(false));

        let inv1 = invoked.clone();
        let inv2 = invoked.clone();
        let inv3 = invoked.clone();
        let inv4 = invoked.clone();
        let inv5 = invoked.clone();

        let actions = KeybindingActions {
            on_archive: Rc::new(move || inv1.store(true, Ordering::SeqCst)),
            on_trash: Rc::new(|| {}),
            on_reply: Rc::new(|| {}),
            on_reply_all: Rc::new(|| {}),
            on_forward: Rc::new(|| {}),
            on_star: Rc::new(|| {}),
            on_toggle_read: Rc::new(|| {}),
            on_focus_search: Rc::new(|| {}),
            on_back: Rc::new(|| {}),
            on_sync: Rc::new(move || inv2.store(true, Ordering::SeqCst)),
            on_show_shortcuts: Rc::new(move || inv3.store(true, Ordering::SeqCst)),
            on_settings: Rc::new(move || inv5.store(true, Ordering::SeqCst)),
            on_undo: Rc::new(move || inv4.store(true, Ordering::SeqCst)),
            on_nav_folder: Rc::new(|_| {}),
        };

        (actions.on_archive)();
        assert!(invoked.load(Ordering::SeqCst));

        invoked.store(false, Ordering::SeqCst);
        (actions.on_sync)();
        assert!(invoked.load(Ordering::SeqCst));

        invoked.store(false, Ordering::SeqCst);
        (actions.on_show_shortcuts)();
        assert!(invoked.load(Ordering::SeqCst));

        invoked.store(false, Ordering::SeqCst);
        (actions.on_settings)();
        assert!(invoked.load(Ordering::SeqCst));

        invoked.store(false, Ordering::SeqCst);
        (actions.on_undo)();
        assert!(invoked.load(Ordering::SeqCst));
    }
}
