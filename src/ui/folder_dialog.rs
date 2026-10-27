// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::Path;
use adw::prelude::*;

/// Displays a modal dialog prompting the user for a new Maildir folder name.
/// Validates input, creates the folder on disk, and fires `on_created`.
pub fn prompt_create_folder<F>(
    parent: &impl IsA<gtk::Widget>,
    account_root: &Path,
    on_created: F,
) where
    F: Fn(String) + 'static,
{
    let dialog = adw::Dialog::builder()
        .title("New Maildir Folder")
        .content_width(340)
        .content_height(190)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 10);
    vbox.set_margin_start(16);
    vbox.set_margin_end(16);
    vbox.set_margin_top(16);
    vbox.set_margin_bottom(16);

    let title_label = gtk::Label::builder()
        .label("Create New Folder")
        .xalign(0.0)
        .build();
    title_label.add_css_class("title-3");
    vbox.append(&title_label);

    let error_label = gtk::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .visible(false)
        .build();
    error_label.add_css_class("error");
    vbox.append(&error_label);

    let entry = gtk::Entry::builder()
        .placeholder_text("Folder name (e.g. Work, Projects)")
        .hexpand(true)
        .build();
    vbox.append(&entry);

    let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    btn_box.set_halign(gtk::Align::End);
    btn_box.set_margin_top(8);

    let cancel_btn = gtk::Button::builder().label("Cancel").build();
    cancel_btn.add_css_class("flat");
    btn_box.append(&cancel_btn);

    let create_btn = gtk::Button::builder().label("Create").build();
    create_btn.add_css_class("suggested-action");
    btn_box.append(&create_btn);

    vbox.append(&btn_box);
    dialog.set_child(Some(&vbox));

    let dialog_cancel = dialog.clone();
    cancel_btn.connect_clicked(move |_| {
        dialog_cancel.close();
    });

    let dialog_submit = dialog.clone();
    let account_root_buf = account_root.to_path_buf();
    let entry_submit = entry.clone();
    let err_submit = error_label.clone();

    let do_create = move || {
        let name = entry_submit.text().trim().to_string();
        if name.is_empty() {
            err_submit.set_text("Folder name cannot be empty");
            err_submit.set_visible(true);
            return;
        }
        if name.contains('/') || name.contains('\\') || name.starts_with('.') {
            err_submit.set_text("Folder name cannot contain slashes or begin with a dot");
            err_submit.set_visible(true);
            return;
        }
        let standard = ["INBOX", "Archive", "Trash", "Sent", "Drafts"];
        if standard.iter().any(|&s| s.eq_ignore_ascii_case(&name)) {
            err_submit.set_text("Cannot use a reserved system mailbox name");
            err_submit.set_visible(true);
            return;
        }

        let folder_path = account_root_buf.join(&name);
        if folder_path.exists() {
            err_submit.set_text("A folder with this name already exists");
            err_submit.set_visible(true);
            return;
        }

        if let Err(e) = crate::services::maildir::ensure_maildir_folder(&folder_path) {
            err_submit.set_text(&format!("Failed to create folder: {e}"));
            err_submit.set_visible(true);
            return;
        }

        on_created(name);
        dialog_submit.close();
    };

    let do_create_rc = std::rc::Rc::new(do_create);
    let do_create_1 = do_create_rc.clone();
    create_btn.connect_clicked(move |_| {
        do_create_1();
    });

    let do_create_2 = do_create_rc.clone();
    entry.connect_activate(move |_| {
        do_create_2();
    });

    dialog.present(Some(parent));
    entry.grab_focus();
}

/// Displays a modal dialog prompting the user to rename an existing Maildir folder.
/// Validates input, renames the folder on disk, and fires `on_renamed`.
pub fn prompt_rename_folder<F>(
    parent: &impl IsA<gtk::Widget>,
    account_root: &Path,
    current_name: &str,
    on_renamed: F,
) where
    F: Fn(String, String) + 'static,
{
    let dialog = adw::Dialog::builder()
        .title("Rename Folder")
        .content_width(340)
        .content_height(190)
        .build();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 10);
    vbox.set_margin_start(16);
    vbox.set_margin_end(16);
    vbox.set_margin_top(16);
    vbox.set_margin_bottom(16);

    let title_label = gtk::Label::builder()
        .label(&format!("Rename \"{current_name}\""))
        .xalign(0.0)
        .build();
    title_label.add_css_class("title-3");
    vbox.append(&title_label);

    let error_label = gtk::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .visible(false)
        .build();
    error_label.add_css_class("error");
    vbox.append(&error_label);

    let entry = gtk::Entry::builder()
        .text(current_name)
        .hexpand(true)
        .build();
    vbox.append(&entry);

    let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    btn_box.set_halign(gtk::Align::End);
    btn_box.set_margin_top(8);

    let cancel_btn = gtk::Button::builder().label("Cancel").build();
    cancel_btn.add_css_class("flat");
    btn_box.append(&cancel_btn);

    let rename_btn = gtk::Button::builder().label("Rename").build();
    rename_btn.add_css_class("suggested-action");
    btn_box.append(&rename_btn);

    vbox.append(&btn_box);
    dialog.set_child(Some(&vbox));

    let dialog_cancel = dialog.clone();
    cancel_btn.connect_clicked(move |_| {
        dialog_cancel.close();
    });

    let dialog_submit = dialog.clone();
    let account_root_buf = account_root.to_path_buf();
    let old_name = current_name.to_string();
    let entry_submit = entry.clone();
    let err_submit = error_label.clone();

    let do_rename = move || {
        let new_name = entry_submit.text().trim().to_string();
        if new_name == old_name {
            dialog_submit.close();
            return;
        }

        match crate::services::maildir::rename_folder(&account_root_buf, &old_name, &new_name) {
            Ok(()) => {
                on_renamed(old_name.clone(), new_name);
                dialog_submit.close();
            }
            Err(e) => {
                err_submit.set_text(&e);
                err_submit.set_visible(true);
            }
        }
    };

    let do_rename_rc = std::rc::Rc::new(do_rename);
    let do_rename_1 = do_rename_rc.clone();
    rename_btn.connect_clicked(move |_| {
        do_rename_1();
    });

    let do_rename_2 = do_rename_rc.clone();
    entry.connect_activate(move |_| {
        do_rename_2();
    });

    dialog.present(Some(parent));
    entry.grab_focus();
}

/// Displays an `adw::AlertDialog` confirmation before deleting a custom Maildir folder.
/// Messages in the folder are moved to Trash first. Fires `on_deleted(folder_name, moved_count)`.
pub fn prompt_delete_folder<F>(
    parent: &impl IsA<gtk::Widget>,
    account_root: &Path,
    folder_name: &str,
    on_deleted: F,
) where
    F: Fn(String, usize) + 'static,
{
    let alert = adw::AlertDialog::builder()
        .heading(&format!("Delete folder \"{folder_name}\"?"))
        .body(&format!(
            "All messages in \"{folder_name}\" will be safely moved to Trash before the folder is removed."
        ))
        .build();

    alert.add_response("cancel", "Cancel");
    alert.add_response("delete", "Delete Folder");
    alert.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

    let account_root_buf = account_root.to_path_buf();
    let folder_to_delete = folder_name.to_string();

    alert.choose(Some(parent), None::<&gio::Cancellable>, move |response| {
        if response == "delete" {
            match crate::services::maildir::delete_folder(&account_root_buf, &folder_to_delete) {
                Ok(moved_count) => {
                    on_deleted(folder_to_delete, moved_count);
                }
                Err(e) => {
                    tracing::error!("Failed to delete folder: {e}");
                }
            }
        }
    });
}
