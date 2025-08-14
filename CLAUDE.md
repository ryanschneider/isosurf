# Isosurf - Isometric Surfing Game

## Project Overview
An isometric surfing game built with Bevy 0.16. The project has completed Phase 2: implementing realistic Gerstner wave simulation with surfboard physics.

## Phase 1: Water System Implementation ✅ (Legacy - Sine Waves)

### Architecture
- **Main module**: `src/water.rs` contains all water simulation logic
- **Plugin system**: `WaterPlugin` encapsulates all water-related functionality
- **Component-based**: Uses Bevy ECS with `WaterSurface` and `WaterWaves` components

### Technical Implementation
- **Mesh**: 200x200 vertex grid (~40,000 vertices) covering 100x100 world units
- **Wave simulation**: 4 sine waves with different parameters combine for realistic motion
- **Performance**: Achieved ~145 FPS on M3 Pro (target was 60 FPS)
- **Camera**: Isometric view at 26.5° angle with 45° rotation
- **Update frequency**: Uses `FixedUpdate` for consistent wave animation

## Phase 2: Gerstner Wave Upgrade ✅

### Major Improvements
- **Upgraded from sine waves to Gerstner waves** for realistic ocean motion with sharp crests and broad troughs
- **4-wave optimized system** with unified flow direction and larger wavelengths for natural-looking waves
- **SIMD optimization** using `wide` crate for 4x parallel vertex processing
- **Fast height query system** for surfboard physics integration
- **Surfboard physics foundation** with buoyancy and floating mechanics

### Gerstner Wave Technical Details
- **Horizontal displacement**: Creates characteristic sharp wave crests via `Q * A * cos(phase) * direction`
- **Vertical displacement**: Standard wave height via `A * sin(phase)`
- **Steepness control**: Q parameter (0.0-1.0) with validation to prevent over-steep waves
- **Pre-calculated wave numbers**: k = 2π/L stored for performance

### Current Wave Parameters (4 Optimized Gerstner Waves)
```rust
// Unified left-to-right flow with larger wavelengths for flatter, more realistic waves
Wave 1: A=1.0,  L=25.0, speed=2.0, dir=(1.0,0.1),  Q=0.15  // Primary large wave
Wave 2: A=0.6,  L=18.0, speed=1.8, dir=(0.9,0.2),  Q=0.18  // Medium wave  
Wave 3: A=0.4,  L=12.0, speed=2.2, dir=(1.1,-0.1), Q=0.20  // Detail wave
Wave 4: A=0.25, L=8.0,  speed=2.5, dir=(0.8,0.3),  Q=0.15  // Surface texture
```

### Key Code Structure
- `WaterParameters`: Enhanced with steepness (Q) and pre-calculated wave_number (k)
- `calculate_gerstner_displacement()`: Full 3D displacement calculation
- `calculate_gerstner_displacement_simd()`: SIMD-optimized version for 4 vertices at once
- `get_wave_height()`: Fast height-only query for physics systems
- `FloatingBody` component: Buoyancy physics with multiple sampling points
- `Surfboard` component: Physical surfboard properties

### Performance Optimizations
- **SIMD processing**: 4 vertices calculated in parallel using f32x4
- **Pre-calculated constants**: Wave numbers stored to avoid repeated 2π/L calculations
- **Chunked processing**: Vertices processed in groups of 4 with scalar fallback
- **Height-only queries**: Separate fast path for physics systems that only need Y displacement

### Surfboard Physics System
- **Multi-point buoyancy**: Samples water height at 5 points (4 corners + center)
- **Archimedes principle**: Buoyancy force based on submerged volume
- **Dynamic tilting**: Torque calculation based on wave slopes
- **Drag damping**: Prevents excessive bouncing and spinning
- **Realistic density**: Surfboard (200 kg/m³) vs water (1000 kg/m³)

### Performance Results
- **144-153 FPS** consistently on M3 Pro (far exceeding 60fps target)
- **40,000 vertices** updated with Gerstner displacement every frame
- **4 optimized waves** with SIMD acceleration and unified flow direction
- **Surfboard physics** running concurrently with wave simulation

### Build & Run
```bash
cargo run  # Builds and runs with 144-153 FPS on M3 Pro
```

### Phase 2 Lessons Learned
1. **Gerstner wave realism**: Horizontal displacement creates the sharp crests that make waves look natural. The Q (steepness) parameter is critical for preventing unrealistic over-steep waves.

2. **SIMD optimization**: Processing 4 vertices at once with wide::f32x4 provided significant performance gains, especially with 8 complex wave calculations per vertex.

3. **Buoyancy physics**: Multi-point sampling is essential for realistic floating behavior. Single-point sampling creates unrealistic motion.

4. **Performance headroom**: Even with doubled wave complexity and physics simulation, we achieved 144-153 FPS, leaving room for gameplay features.

5. **Wave parameter tuning**: The 4-wave setup with unified left-to-right flow and larger wavelengths (8-25 units) creates natural, flatter ocean waves without the chaotic peaks that shorter wavelengths produced.

## Next Phase Planning
Phase 3 should focus on:
- Player controls for surfboard movement
- Wave riding mechanics and scoring
- Particle effects (spray, foam)
- Advanced water shading and materials
- Game objectives and progression

## Development Commands
- `cargo run` - Run the application
- `cargo check` - Quick compilation check
- FPS counter logs to console automatically