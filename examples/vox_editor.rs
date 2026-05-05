//! An editor for .vox files

use bevy::{
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow, Window, WindowMode},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(VoxelPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, movement)
        .add_systems(Update, build_break)
        .run();
}
