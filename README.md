# Rust GL Water

A high-performance, feature-rich port of the classic [WebGL Water](http://madebyevan.com/webgl-water/) simulation to Rust, powered by **wgpu** and GPGPU computing.

![Demo](demo.gif)

## 🌟 Features

### 🌊 Advanced Water Simulation
- **Real-time Wave Equation**: High-fidelity simulation using a 9-point Laplacian on a 512x512 GPU texture grid.
- **Dynamic Caustics**: Real-time light refraction patterns on the pool floor that react to every ripple and object.
- **Surface Ripples**: 
  - **Impact Splashes**: Objects create splashes when hitting the water.
  - **Sinking Ripples**: Dense objects create continuous small ripples as they sink.

### 🧊 Physics & Interaction
- **Density-Based Buoyancy**: Objects float or sink based on their material density:
  - **Wood**: Floats high (Density 0.6)
  - **Ice**: Floats low (Density 0.92)
  - **Glass/Steel**: Sinks (Density 2.5 / 7.8)
- **Multi-Object System**: Simulate up to **10 concurrent objects** with full collision detection.
- **Interactive**: Click/drag to create ripples or move submerged objects with realistic physics.

### 🎨 Visuals & Materials
- **Advanced Materials**:
  - **Glass**: High refraction with Fresnel-modulated transparency.
  - **Steel**: Metallic reflections.
  - **Wood**: Opaque with diffuse lighting.
  - **Ice**: Frosted appearance.
- **Diverse Geometry**: Support for **Spheres, Tori, Tetrahedrons, and Cubes**.
- **Customizable Pool**: Change pool shape to **Cube, Cuboid, Frustum, or Tube**.

### 🛠️ Configuration
- **Real-time GUI**: Adjust gravity, light color, object count, and more on the fly.
- **Backend Selection**: Choose between **Vulkan, OpenGL, DirectX 11/12**, or Auto-detect.

## 🎮 Controls

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

## 🏗️ Architecture

The project leverages **wgpu** for cross-platform GPU acceleration and features a modular architecture:

```
src/
├── core/            # Core abstractions (Shape, Material, Physics traits)
├── shapes/          # Shape implementations (Sphere, Torus, Cube, etc.)
├── physics.rs       # Physics engine (Buoyancy, Collision, Ripples)
├── water.rs         # GPU ping-pong wave simulation
├── renderer.rs      # Main render pipeline
├── app.rs           # Application state & event loop
└── shaders/         # WGSL shaders (externalized)
```

### Key Algorithms

- **GPGPU Wave Simulation**: Solved on a grid using ping-pong textures for isotropic wave propagation.
- **Semi-Implicit Euler Physics**: Force-based integration with density-dependent buoyancy and drag.
- **Ray-Marched SDFs**: Complex geometry intersection using Signed Distance Fields.
- **Volume Displacement**: Analytical and ray-marched volume displacement for accurate water interaction.

## 🚀 Building & Running

### Prerequisites
- **Rust 1.70+**: [rustup.rs](https://rustup.rs)
- **Modern GPU**: Supports Vulkan, DirectX 12, or Metal.

### Run
```bash
# Release build (highly recommended for fluid 60+ FPS)
cargo run --release
```

## 📜 Credits
- **Evan Wallace**: Original [WebGL Water](http://madebyevan.com/webgl-water/) creator.
- **Del Wang**: [Idootop](https://github.com/idootop) for the WebGL2 reference implementation.

## 📄 License
MIT License
