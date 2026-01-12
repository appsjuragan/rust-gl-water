# Rust GL Water

A high-performance port of the classic [WebGL Water](http://madebyevan.com/webgl-water/) simulation to Rust, powered by **wgpu** and GPGPU computing.

![Demo](demo.gif)

## Features

- 🌊 **Real-time Water Simulation**: High-fidelity wave equation simulation using a 9-point Laplacian on a 512x512 GPU texture grid.
- 🧊 **Multi-Object System**: Simulate and interact with up to **5 concurrent objects** simultaneously with full collision detection.
- 💎 **Advanced Materials**: Realistic material simulation with dynamic refraction and reflection:
  - **Glass**: High refraction with Fresnel-modulated transparency.
  - **Steel**: Metallic reflections tinted by material color.
  - **Wood**: Opaque material with diffuse lighting.
  - **Ice**: Frosted appearance with subtle refraction.
- ✨ **Dynamic Caustics**: Real-time light refraction patterns on the pool floor that react to every ripple and object.
- 📐 **Diverse Geometry**: Support for multiple 3D primitives including **Spheres, Tori, Tetrahedrons, and Cubes**.
- 🛠️ **Real-time Configuration**: Integrated GUI (egui) for adjusting gravity, object count, materials, and lighting on the fly.
- 🎯 **Interaction**: Click/drag to create ripples or move submerged objects with realistic buoyancy physics.
- 🎮 **Orbit & Zoom Camera**: Intuitive orbital camera with smooth zoom.

## Controls

| Action | Effect |
|--------|--------|
| **Left Click + Drag on Water** | Create ripples |
| **Left Click + Drag on Object** | Move object (buoyancy active) |
| **Left Click + Drag on Wall** | Orbit camera |
| **Mouse Scroll** | Zoom in/out |
| **Space** | Pause/Resume simulation |
| **'G' Key** | Toggle gravity |
| **'L' Key** | Update light direction to view |
| **'P' Key** | Toggle FPS display |
| **'O' Key** | Open configuration panel |
| **Escape** | Exit |

## Building

### Prerequisites

- **Rust 1.70+**: [rustup.rs](https://rustup.rs)
- **Modern GPU**: Supports Vulkan, DirectX 12, or Metal.

### Build and Run

```bash
# Release build (highly recommended for fluid 60+ FPS)
cargo run --release
```

## Architecture

The project leverages **wgpu** for cross-platform GPU acceleration:

```
src/
├── main.rs          # Entry point
├── app.rs           # Application state, event loop, fixed-timestep update
├── camera.rs        # Orbital camera with projection matrices
├── physics.rs       # Semi-implicit Euler integration, buoyancy, collisions
├── water.rs         # GPU ping-pong wave simulation orchestration
├── renderer.rs      # Main render pipeline, mesh generation, uniform management
├── input.rs         # Mouse/keyboard handling, raycasting
├── ui.rs            # Simple FPS overlay renderer
├── gui.rs           # egui-based configuration panel
└── shaders/
    ├── common.rs       # Shared SDF utilities and material definitions
    ├── water_sim.rs    # GPGPU wave propagation and volume displacement
    ├── water_render.rs # Water surface fragment shader with refraction
    ├── caustics.rs     # Caustic light pattern generation
    ├── sphere.rs       # Material-aware object surface shader
    └── pool.rs         # Pool walls/floor shader
```

## Key Algorithms

### Fixed-Timestep Simulation
Physics and water simulation run at a locked **60Hz** using an accumulator pattern, ensuring consistent behavior regardless of frame rate. This eliminates animation stutter and provides deterministic results.

### 2D Wave Equation
Solved on a 512x512 grid using ping-pong textures with a **9-point Laplacian** for isotropic wave propagation. This produces smoother, more realistic ripples than a standard 4-point stencil.

### Semi-Implicit Euler Physics
Objects use force-based integration with:
- Buoyancy calculated from submerged volume
- Quadratic drag scaling with submersion depth
- Velocity damping to prevent energy accumulation
- Elastic collision resolution between objects

### Ray-Marched SDFs
Shapes like Torus and Tetrahedron are intersected using optimized Signed Distance Fields, enabling complex geometry without explicit mesh intersection.

### Volume Displacement
Submerged objects displace water volume analytically (for spheres, cubes, tori) or via ray-marching (tetrahedrons), creating realistic waves when moved through the water surface.

## Performance Notes

- **GPU-bound**: The simulation is primarily limited by fragment shader complexity.
- **60+ FPS**: Achievable on most discrete GPUs at 600x600 resolution.
- **VSync**: Enabled by default (`PresentMode::AutoVsync`).

## Credits

- **Evan Wallace**: Original [WebGL Water](http://madebyevan.com/webgl-water/) creator.
- **Del Wang**: [Idootop](https://github.com/idootop) for the WebGL2 reference implementation.

## License

MIT License
