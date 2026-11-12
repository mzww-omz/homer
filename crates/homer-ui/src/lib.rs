//! GPUI application frontend for Homer.

pub mod app;
mod modals;
mod render;
pub mod services;
pub mod text_area;
pub mod theme;
pub mod ui;

pub use app::{
    bind_keys, minimum_window_size, window_size, HomerApp, Modal, SettingsForm, Startup,
    WINDOW_TITLE,
};
pub use services::{Services, UiEvent};
