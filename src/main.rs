//! # Rust GL Water
//! 
//! A port of the WebGL2 Water simulation to Rust using wgpu.
//! Original: https://github.com/idootop/webgl2-water

// Hide console in release mode on Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// New architecture modules
mod core;
mod shapes;
mod scene;

// Existing modules

mod app;
mod water;
mod ui;
mod renderer;
mod physics;
mod input;
mod camera;
mod shaders;
mod gui;

use app::Application;
use winit::event_loop::EventLoop;

fn main() {
    // Initialize logging
    env_logger::init();
    log::info!("Starting Rust GL Water Simulation");

    // Create event loop and run application
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    
    let mut app = pollster::block_on(Application::new(&event_loop));
    
    event_loop.run_app(&mut app).expect("Event loop error");
}
