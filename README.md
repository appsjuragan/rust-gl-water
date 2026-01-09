# Rust GL Water

A high-performance port of the classic [WebGL Water](http://madebyevan.com/webgl-water/) simulation to Rust, powered by **wgpu** and GPGPU computing.

![Demo](demo.gif)

## Features

- 🌊 **Real-time Water Simulation**: High-fidelity wave equation simulation using GPU compute (fragment shader fallback).
- 🧊 **Multi-Object System**: Simulate and interact with up to **5 concurrent objects** simultaneously.
- 💎 **Advanced Materials**: Realistic material simulation with dynamic refraction and reflection:
  - **Glass**: High refraction with light absorption.
  - **Steel**: Metallic reflections tinted by material color.
  - **Wood**: Opaque material with diffuse lighting.
  - **Ice**: Frosted appearance with subtle refraction.
- ✨ **Dynamic Caustics**: Real-time light refraction patterns on the pool floor that react to every ripple and object.
- 📐 **Diverse Geometry**: Support for multiple 3D primitives including **Spheres, Tori, Tetrahedrons, and Cubes**.
- 🛠️ **Real-time Configuration**: Integrated GUI for adjusting gravity, object count, materials, and lighting on the fly.
- 🎯 **Interaction**: Click/drag to create ripples or move submerged objects with realistic buoyancy physics.
- 🎮 **Orbit & Zoom Camera**: Intuitive orbital camera with smooth zoom.

## Controls

| Action | Effect |
|--------|--------|
| **Left Click + Drag on Water** | Create ripples |
| **Left Click + Drag on Object** | Move object (Buoyancy active) |
| **Left Click + Drag on Wall** | Orbit camera |
| **Mouse Scroll** | Zoom in/out |
| **Space** | Pause/Resume simulation |
| **'G' Key** | Toggle gravity |
| **'L' Key** | Update light direction to view |
| **'P' Key** | Toggle FPS display |
| **Escape** | Exit |

## Building

### Prerequisites

- **Rust 1.70+**: [rustup.rs](https://rustup.rs)
- **Modern GPU**: Supports Vulkan, DirectX 12, or Metal.

### Build and Run

```bash
# Release build (highly recommended for fluid 60 FPS)
cargo run --release
```

## Architecture

The project leverages **wgpu** for cross-platform GPU acceleration:

- **src/physics.rs**: Handles buoyancy and multi-object collision states.
- **src/water.rs**: Orchestrates GPU textures for ping-pong wave simulation.
- **src/renderer.rs**: Manages the main rendering pipeline and instanced object rendering.
- **src/shaders/**: Optimized WGSL shaders.
  - `common.rs`: Shared SDF utilities and unrolled light/logic loops for maximum compatibility.
  - `water_sim.rs`: GPGPU logic for wave propagation and volume displacement.
  - `sphere.rs`: Material-aware shader for object rendering with support for instancing.

## Key Algorithms

1. **2D Wave Equation**: Solved on a 512x512 grid using ping-pong textures for stable, real-time wave propagation.
2. **Ray-Marched SDFs**: Shapes like Torus and Tetrahedron are intersected using optimized Signed Distance Fields.
3. **Instanced Rendering**: Render multiple objects efficiently by passing arrays of centers to the GPU and utilizing builtin instance IDs.
4. **Volume Displacement**: Submerged objects displace water volume analytically, creating realistic waves when moved.

## Credits

- **Evan Wallace**: Original [WebGL Water](http://madebyevan.com/webgl-water/) creator.
- **Del Wang**: [Idootop](https://github.com/idootop) for the WebGL2 reference implementation.

## License

MIT License
