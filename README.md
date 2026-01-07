# Rust GL Water

A port of the [WebGL2 Water](https://github.com/idootop/webgl2-water) simulation to Rust using wgpu.

![Demo](demo.gif)

## Features

- 🌊 **Real-time Water Simulation**: GPU-accelerated wave equation simulation
- ✨ **Caustic Lighting**: Dynamic light refraction patterns on pool floor
- 🎯 **Interactive**: Click/drag to create ripples
- 🦆 **Floating Objects**: Physics-based buoyancy simulation
- 🎮 **Orbit Camera**: Drag to rotate view around the pool

## Controls

| Action | Effect |
|--------|--------|
| **Left Click + Drag on Water** | Create ripples |
| **Left Click + Drag on Empty Space** | Orbit camera |
| **Space** | Add random drop |
| **Escape** | Exit |

## Building

### Prerequisites

- Rust 1.70+ (install from https://rustup.rs)
- A GPU with Vulkan, DX12, or Metal support

### Build and Run

```bash
# Debug build
cargo run

# Release build (recommended for performance)
cargo run --release
```

### Using MinGW (optional)

If you want to compile with MinGW instead of MSVC:

1. Ensure MinGW is installed at `D:\projects\mingw64`
2. Uncomment the target line in `.cargo/config.toml`
3. Add the MinGW GNU target:
   ```bash
   rustup target add x86_64-pc-windows-gnu
   ```
4. Build:
   ```bash
   cargo build --target x86_64-pc-windows-gnu --release
   ```

## Architecture

The project is structured as follows:

```
src/
├── main.rs          # Entry point
├── app.rs           # Application & event loop
├── camera.rs        # Orbital camera
├── input.rs         # Mouse/keyboard input handling
├── physics.rs       # Buoyancy physics simulation
├── water.rs         # GPU water simulation
├── renderer.rs      # Scene rendering
└── shaders/         # WGSL shaders
    ├── common.rs    # Shared constants & functions
    ├── water_sim.rs # Wave simulation shaders
    ├── water_render.rs # Water surface shader
    ├── caustics.rs  # Light caustics shader
    └── pool.rs      # Pool walls shader
```

### Key Algorithms

1. **Water Simulation**: 2D wave equation solved on GPU using ping-pong textures
2. **Caustics**: Light refraction patterns calculated per-vertex and rendered additively
3. **Fresnel Effect**: Realistic reflection/refraction mixing based on view angle
4. **Buoyancy**: Simple floating object physics with water interaction

## Credits

Original WebGL2 implementation by [Del Wang](https://github.com/idootop):
- https://github.com/idootop/webgl2-water

Based on the classic WebGL Water by [Evan Wallace](http://madebyevan.com):
- http://madebyevan.com/webgl-water/

## License

MIT License
