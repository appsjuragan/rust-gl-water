# Refactoring Plan: Modular & Dynamic Architecture

## Objective
Deep analysis and refactoring of the Rust GL Water codebase to achieve:
1.  **Abstract Separation**: Decouple object shapes, pool shapes, physics, and GUI.
2.  **Dynamic Extensibility**: Allow adding objects/shapes without breaking physics or other code.
3.  **Efficiency**: Remove hardcoded values, duplicate state, and redundant mesh types.
4.  **Portability**: Centralize platform/backend configuration.

## Phase 1: Core Unification & definitions (Day 1)
**Goal**: Establish a single source of truth for types and constants.

1.  [x] **Create `src/core/constants.rs`**: Move all hardcoded magic numbers (gravity defaults, pool dimensions, damping factors) here.
2.  [x] **Create `src/core/enums.rs`**: Centralize `Shape`, `Texture`, `PoolShape`, `Backend` enums used across GUI, Renderer, and Physics.
3.  [x] **Create `src/core/geometry.rs`**: Define a unified `Vertex` struct to replace duplicate `WaterVertex`, `PoolVertex`, and `ShapeVertex`.
4.  [x] **Create `src/core/state.rs`**: Unify `ObjectState` (physics impl) and `PhysicsState` (trait) into a single, comprehensive `RigidBody` struct.

## Phase 2: Decoupling GUI & State (Day 1)
**Goal**: GUI should only modify independent configuration state, not define domain types.

1.  **Refactor `gui.rs`**: 
    - Remove enum definitions.
    - Update to use `core::enums` and `core::config`.
    - Extract `AppConfig` to `src/core/config.rs` (or `settings.rs`).

## Phase 3: World & Physics Abstraction (Day 2)
**Goal**: Make the physical world (Pool, Gravity) dynamic objects.

1.  **Create `src/world/` module**:
    - `pool.rs`: Abstract pool geometry logic (walls, collision boundaries) into a trait/struct system.
    - `environment.rs`: Manage global forces (gravity, wind, etc.).
2.  **Refactor `physics.rs`**:
    - Remove hardcoded pool collision logic.
    - Delegate collision checks to the generic `Pool` abstraction.
    - Update to use the unified `RigidBody` struct.

## Phase 4: Dynamic Object System (Day 2)
**Goal**: Allow adding new objects/shapes at runtime without changing the renderer.

1.  [x] **Update `renderer.rs`**:
    - [x] Use unified `Vertex` type.
    - Implement a `Renderable` trait that objects implement.
    - Remove big `match` statements for shapes; use a collection of objects.
2.  **Refactor `object_manager.rs`**:
    - Properly utilize this to manage the list of active objects in the scene.

## Phase 5: Cleanup & Optimization (Day 3)
1.  **Remove duplicate code**: Clean up `physics_trait.rs` if it becomes redundant.
2.  **Optimize Shaders**: Ensure shaders use the new uniform structures.
3.  **Final Polish**: Verify "Wow" aesthetics and ensure all features work with the new architecture.

---

## Detailed Steps for Immediate Execution (Phase 1)

1.  Generate `src/core/constants.rs`
2.  Generate `src/core/enums.rs`
3.  Generate `src/core/geometry.rs`
4.  Generate `src/core/state.rs`
5.  Refactor `gui.rs` to use these new core modules.
