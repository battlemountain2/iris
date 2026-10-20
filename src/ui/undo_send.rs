#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

pub fn show_undo_toast<F>(overlay: &adw::ToastOverlay, recipient: &str, on_undo: F)
where
    F: Fn() + 'static,
{
    let target = if recipient.is_empty() {
        "recipient".to_string()
    } else {
        recipient.to_string()
    };

    // Note: Once the persistent outbox and SMTP service are wired, this will state
    // "Sending to {target}... (Undo 10s)". While SMTP transport is under development,
    // we honestly report that the draft was saved locally.
    let toast = adw::Toast::builder()
        .title(format!("Draft saved: {target} (SMTP queue pending)"))
        .button_label("Undo")
        .timeout(10)
        .priority(adw::ToastPriority::High)
        .build();

    toast.connect_button_clicked(move |_| {
        tracing::info!("Undo clicked! Canceling send dispatch.");
        on_undo();
    });

    overlay.add_toast(toast);
}

pub fn show_undo_action_toast<F>(overlay: &adw::ToastOverlay, title: &str, on_undo: F)
where
    F: Fn() + 'static,
{
    let toast = adw::Toast::builder()
        .title(title)
        .button_label("Undo")
        .timeout(10)
        .priority(adw::ToastPriority::High)
        .build();

    let title_str = title.to_string();
    toast.connect_button_clicked(move |_| {
        tracing::info!("Undo clicked for: {title_str}");
        on_undo();
    });

    overlay.add_toast(toast);
}
