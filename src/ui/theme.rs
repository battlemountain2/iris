#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
};
use serde::{Deserialize, Serialize};

pub mod noctalia;

const BUILTIN_THEMES: [(&str, &str); 12] = [
    ("iris-dark", include_str!("../../data/themes/attic-olive.toml")),
    ("iris-light", include_str!("../../data/themes/iris-light.toml")),
    ("attic-olive", include_str!("../../data/themes/attic-olive.toml")),
    (
        "corinthian-bronze",
        include_str!("../../data/themes/corinthian-bronze.toml"),
    ),
    ("catppuccin", include_str!("../../data/themes/catppuccin.toml")),
    ("everforest", include_str!("../../data/themes/everforest.toml")),
    ("rose-pine", include_str!("../../data/themes/rose-pine.toml")),
    ("tokyo-night", include_str!("../../data/themes/tokyo-night.toml")),
    ("azure-glow", include_str!("../../data/themes/azure-glow.toml")),
    ("graphite", include_str!("../../data/themes/graphite.toml")),
    ("midnight", include_str!("../../data/themes/midnight.toml")),
    ("paper", include_str!("../../data/themes/paper.toml")),
];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ThemeTokens {
    pub name: String,
    pub background: String,
    pub surface: String,
    #[serde(default)]
    pub reader_surface: Option<String>,
    pub text: String,
    pub accent: String,
    pub danger: String,
    pub muted: String,
    pub highlight: String,
    pub border: String,
    pub dim_text: String,
}

pub struct ThemeManager {
    pub current_id: RefCell<String>,
    pub subtle_mode: RefCell<bool>,
    pub follows_noctalia: RefCell<bool>,
    noctalia: RefCell<noctalia::Palette>,
    css_provider: gtk::CssProvider,
}

impl ThemeManager {
    pub fn new() -> Rc<Self> {
        let config_dir = glib_user_config_dir();
        let noctalia_path = noctalia::palette_path(&config_dir);
        let mut noctalia_palette = noctalia::Palette::new(noctalia_path);
        let _ = noctalia_palette.refresh();

        let css_provider = gtk::CssProvider::new();
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &css_provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 10,
            );
        }

        let manager = Rc::new(Self {
            current_id: RefCell::new("attic-olive".to_string()),
            subtle_mode: RefCell::new(true),
            follows_noctalia: RefCell::new(false),
            noctalia: RefCell::new(noctalia_palette),
            css_provider,
        });

        manager.apply_current();
        manager
    }

    pub fn current_tokens(&self) -> ThemeTokens {
        if *self.follows_noctalia.borrow() {
            if let Some(tokens) = &self.noctalia.borrow().last_valid {
                return tokens.clone();
            }
        }

        let id = self.current_id.borrow();
        for (theme_id, toml_str) in BUILTIN_THEMES {
            if *theme_id == *id {
                if let Ok(tokens) = toml::from_str::<ThemeTokens>(toml_str) {
                    return tokens;
                }
            }
        }

        // Fallback default (Attic Olive)
        toml::from_str::<ThemeTokens>(BUILTIN_THEMES[0].1).expect("builtin attic-olive valid")
    }

    pub fn set_theme(&self, id: &str) {
        *self.current_id.borrow_mut() = id.to_string();
        *self.follows_noctalia.borrow_mut() = false;
        self.apply_current();
    }

    pub fn set_subtle_mode(&self, subtle: bool) {
        *self.subtle_mode.borrow_mut() = subtle;
        self.apply_current();
    }

    pub fn set_follow_noctalia(&self, follow: bool) {
        *self.follows_noctalia.borrow_mut() = follow;
        if follow {
            let _ = self.noctalia.borrow_mut().refresh();
        }
        self.apply_current();
    }

    pub fn apply_current(&self) {
        let tokens = self.current_tokens();
        let reader_surface = tokens.reader_surface.as_deref().unwrap_or(&tokens.surface);

        let css = format!(
            "@define-color theme_bg {};\n\
             @define-color theme_surface {};\n\
             @define-color theme_reader {};\n\
             @define-color theme_text {};\n\
             @define-color theme_dim_text {};\n\
             @define-color theme_muted {};\n\
             @define-color theme_accent {};\n\
             @define-color theme_danger {};\n\
             @define-color theme_highlight {};\n\
             @define-color theme_border {};\n\
             \n\
             .iris-thread-pane,\n\
             navigation-split-view > widget.content-pane .iris-thread-pane {{\n\
                 background-color: @theme_surface;\n\
                 color: @theme_text;\n\
             }}\n\
             \n\
             .iris-reading-pane,\n\
             navigation-split-view > widget.content-pane .iris-reading-pane {{\n\
                 background-color: @theme_reader;\n\
                 color: @theme_text;\n\
             }}\n\
             \n\
             .iris-unread-dot {{\n\
                 background-color: @theme_accent;\n\
                 box-shadow: 0 0 6px alpha(@theme_accent, 0.5);\n\
             }}\n\
             \n\
             .iris-unread-badge {{\n\
                 background-color: @theme_accent;\n\
                 box-shadow: 0 0 6px alpha(@theme_accent, 0.5);\n\
             }}\n\
             \n\
             .iris-star-badge {{\n\
                 color: @theme_highlight;\n\
             }}\n\
             \n\
             .iris-message-row:selected,\n\
             .iris-thread-card:selected {{\n\
                 background-color: alpha(@theme_accent, 0.22);\n\
                 border-color: alpha(@theme_accent, 0.55);\n\
             }}\n\
             \n\
             .iris-sidebar row:selected {{\n\
                 background-color: alpha(@theme_accent, 0.28);\n\
                 border-color: alpha(@theme_accent, 0.60);\n\
                 color: #ffffff;\n\
             }}\n",
            tokens.background,
            tokens.surface,
            reader_surface,
            tokens.text,
            tokens.dim_text,
            tokens.muted,
            tokens.accent,
            tokens.danger,
            tokens.highlight,
            tokens.border,
        );

        self.css_provider.load_from_string(&css);
    }
}

pub fn blend(left: &str, right: &str, amount: f64) -> String {
    let parse = |value: &str| u32::from_str_radix(value.trim_start_matches('#'), 16).ok();
    let (Some(left), Some(right)) = (parse(left), parse(right)) else {
        return right.to_owned();
    };
    let channel = |shift| {
        let a = f64::from((left >> shift) & 0xff_u32);
        let b = f64::from((right >> shift) & 0xff_u32);
        (a + (b - a) * amount).round() as u32
    };
    format!("#{:02x}{:02x}{:02x}", channel(16), channel(8), channel(0))
}

fn glib_user_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(dir)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        PathBuf::from("/home/bry/.config")
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blend_colors() {
        let white = "#ffffff";
        let black = "#000000";
        let mid = blend(black, white, 0.5);
        assert_eq!(mid, "#808080");
    }

    #[test]
    fn test_builtin_themes_load_validly() {
        for (id, toml_str) in BUILTIN_THEMES {
            let tokens = toml::from_str::<ThemeTokens>(toml_str);
            assert!(tokens.is_ok(), "Theme '{}' failed to parse: {:?}", id, tokens.err());
        }
    }
}
