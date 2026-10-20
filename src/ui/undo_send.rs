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

    let toast = adw::Toast::builder()
        .title(format!("Message sent to {target}"))
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
