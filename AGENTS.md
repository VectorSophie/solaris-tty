# solaris-tty Agent Instructions

This document provides essential guidance for developers working with the solaris-tty repository. These are the key operational patterns, conventions, and constraints that distinguish this project from typical Rust applications.

## Project Overview

solaris-tty is a real-time 3D astrophysics simulator running entirely in the terminal. It features:
- Newtonian N-body gravity simulation with velocity-Verlet integrator
- Real-time rendering using ASCII characters and Braille glyphs
- Support for multiple celestial mechanics scenarios
- Interactive TUI with physics tracing and visualization
- Various rendering modes (blocks, ASCII, text) and scale modes

## Key Patterns & Constraints

### 1. Binary Architecture
- Main executable: `solaris-tty` (compiled from `src/main.rs`)
- Library crate: `solaris-tty` (compiled from `src/lib.rs`)
- All scenarios are bundled as TOML strings in the binary via `include_str!()`

### 2. Scenario System
- Scenarios are defined in `assets/scenarios/*.toml`
- Built-in scenarios are compiled into the binary via `SCENARIOS` constant in `src/lib.rs`
- Each scenario defines:
  - Simulation parameters (time_step, substeps, integrator)
  - Relativity corrections (enabled, model, source, targets)
  - Rendering settings (scale, trail_length, show_orbits)
  - Body definitions with physical properties

### 3. Core Modules
- `src/sim/`: Physics engine including gravity, integration, orbital mechanics
- `src/render/`: Terminal rendering with multiple modes (ASCII, blocks, text)
- `src/scenario/`: Scenario loading and parsing
- `src/app/`: Main application loop and TUI handling
- `src/command/`: Command parsing and execution
- `src/trace/`: Physics tracing and diagnostic information

### 4. Build & Test Patterns
- Build with `cargo build` or `cargo run`
- Release builds with `cargo run --release`
- Tests run with `cargo test`
- Specific test files exist for different modules:
  - `tests/command_tests.rs`
  - `tests/physics_tests.rs`
  - `tests/render_tests.rs`
  - `tests/scenario_tests.rs`
  - `tests/cli_tests.rs`

### 5. Command Line Interface
- Default: `solaris-tty` launches interactive TUI
- `--check`: Headless load + orbit classification + energy check
- `--bench`: Headless benchmark of N-body throughput
- `--frame [scene]`: Render single frame to stdout
- `--record [file] [frames] [scene=name]`: Generate asciinema cast
- `--screensaver`: Launch with auto-orbiting camera
- `run [scenario]`: Launch with specific scenario

### 6. Configuration
- Uses TOML format for scenario definitions
- Configuration is embedded in the binary at compile time
- Runtime configuration handled via command-line flags and in-app commands

### 7. Key Files
- `src/main.rs`: Entry point and CLI argument handling
- `src/lib.rs`: Library exports and bundled scenarios
- `assets/scenarios/solar.toml`: Default scenario definition
- `Cargo.toml`: Dependencies and build configuration

## Development Notes

### Testing
- Tests are organized by module
- Physics tests validate numerical accuracy
- Render tests check rendering behavior
- CLI tests ensure command-line interface works correctly

### Performance
- Release builds use LTO and single codegen unit
- High-performance N-body calculations
- Optimized rendering pipeline for terminal output

### Debugging
- Use `--check` for headless validation
- Use `--bench` for performance profiling
- Use `--frame` to render individual frames for debugging
- Use `:set` and `:inspect` commands during interactive sessions

### Extending Scenarios
- Add new TOML files in `assets/scenarios/`
- Update `SCENARIOS` constant in `src/lib.rs`
- Ensure scenario follows the TOML schema defined in the existing files