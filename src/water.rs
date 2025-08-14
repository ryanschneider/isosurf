use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
    },
};
use wide::f32x4;

#[derive(Component, Debug)]
pub struct WaterSurface {
    pub grid_size: usize,
    pub world_size: f32,
    pub vertex_count: usize,
    pub base_positions: Vec<Vec3>,
}

#[derive(Debug, Clone, Copy)]
pub struct WaveParameters {
    pub amplitude: f32,
    pub wavelength: f32,
    pub speed: f32,
    pub direction: Vec2,
    pub steepness: f32, // Q parameter for Gerstner waves (0.0-1.0)
    pub wave_number: f32, // k = 2π/L (pre-calculated for performance)
}

#[derive(Component, Debug)]
pub struct WaterWaves {
    pub waves: Vec<WaveParameters>,
}

impl Default for WaterWaves {
    fn default() -> Self {
        let mut waves = vec![
            // Wave 1: Large primary wave flowing left-to-right
            WaveParameters {
                amplitude: 1.0,
                wavelength: 25.0,
                speed: 2.0,
                direction: Vec2::new(1.0, 0.1).normalize(),
                steepness: 0.15,
                wave_number: 2.0 * std::f32::consts::PI / 25.0,
            },
            // Wave 2: Medium wave with slight angle variation
            WaveParameters {
                amplitude: 0.6,
                wavelength: 18.0,
                speed: 1.8,
                direction: Vec2::new(0.9, 0.2).normalize(),
                steepness: 0.18,
                wave_number: 2.0 * std::f32::consts::PI / 18.0,
            },
            // Wave 3: Smaller wave for detail
            WaveParameters {
                amplitude: 0.4,
                wavelength: 12.0,
                speed: 2.2,
                direction: Vec2::new(1.1, -0.1).normalize(),
                steepness: 0.2,
                wave_number: 2.0 * std::f32::consts::PI / 12.0,
            },
            // Wave 4: Smallest wave for surface texture
            WaveParameters {
                amplitude: 0.25,
                wavelength: 8.0,
                speed: 2.5,
                direction: Vec2::new(0.8, 0.3).normalize(),
                steepness: 0.15,
                wave_number: 2.0 * std::f32::consts::PI / 8.0,
            },
        ];

        // Validate steepness to prevent over-steep waves (Q * A * k should be < 1.0)
        for wave in &mut waves {
            let max_steepness = 0.9 / (wave.amplitude * wave.wave_number);
            if wave.steepness > max_steepness {
                wave.steepness = max_steepness;
            }
        }

        Self { waves }
    }
}

pub fn create_water_mesh(grid_size: usize, world_size: f32) -> (Mesh, Vec<Vec3>) {
    let vertex_count = grid_size * grid_size;
    let mut positions = Vec::with_capacity(vertex_count);
    let mut normals = Vec::with_capacity(vertex_count);
    let mut uvs = Vec::with_capacity(vertex_count);
    let mut base_positions = Vec::with_capacity(vertex_count);
    
    let step = world_size / (grid_size - 1) as f32;
    let half_size = world_size / 2.0;
    
    for z in 0..grid_size {
        for x in 0..grid_size {
            let x_pos = x as f32 * step - half_size;
            let z_pos = z as f32 * step - half_size;
            let pos = Vec3::new(x_pos, 0.0, z_pos);
            
            positions.push([pos.x, pos.y, pos.z]);
            base_positions.push(pos);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([x as f32 / (grid_size - 1) as f32, z as f32 / (grid_size - 1) as f32]);
        }
    }
    
    let mut indices = Vec::new();
    for z in 0..grid_size - 1 {
        for x in 0..grid_size - 1 {
            let idx = z * grid_size + x;
            
            indices.push(idx as u32);
            indices.push((idx + grid_size) as u32);
            indices.push((idx + 1) as u32);
            
            indices.push((idx + 1) as u32);
            indices.push((idx + grid_size) as u32);
            indices.push((idx + grid_size + 1) as u32);
        }
    }
    
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    
    (mesh, base_positions)
}

/// Calculate Gerstner wave displacement at a given position and time
/// Returns (horizontal_x, horizontal_z, vertical_y) displacement
fn calculate_gerstner_displacement(position: Vec2, wave: &WaveParameters, time: f32) -> (f32, f32, f32) {
    let dot_product = position.dot(wave.direction);
    let phase = wave.wave_number * dot_product - wave.speed * time;
    let cos_phase = phase.cos();
    let sin_phase = phase.sin();
    
    // Horizontal displacement (creates the sharp crests)
    let q_a_cos = wave.steepness * wave.amplitude * cos_phase;
    let horizontal_x = q_a_cos * wave.direction.x;
    let horizontal_z = q_a_cos * wave.direction.y;
    
    // Vertical displacement
    let vertical_y = wave.amplitude * sin_phase;
    
    (horizontal_x, horizontal_z, vertical_y)
}

/// SIMD-optimized Gerstner wave calculation for 4 positions at once
fn calculate_gerstner_displacement_simd(
    positions_x: f32x4,
    positions_z: f32x4,
    wave: &WaveParameters,
    time: f32,
) -> (f32x4, f32x4, f32x4) {
    // Calculate dot products for 4 positions
    let dir_x = f32x4::splat(wave.direction.x);
    let dir_z = f32x4::splat(wave.direction.y);
    let dot_products = positions_x * dir_x + positions_z * dir_z;
    
    // Calculate phases
    let wave_number = f32x4::splat(wave.wave_number);
    let speed_time = f32x4::splat(wave.speed * time);
    let phases = wave_number * dot_products - speed_time;
    
    // Calculate sin and cos of phases
    let sin_phases = phases.sin();
    let cos_phases = phases.cos();
    
    // Horizontal displacement
    let q_a = f32x4::splat(wave.steepness * wave.amplitude);
    let q_a_cos = q_a * cos_phases;
    let horizontal_x = q_a_cos * dir_x;
    let horizontal_z = q_a_cos * dir_z;
    
    // Vertical displacement
    let amplitude = f32x4::splat(wave.amplitude);
    let vertical_y = amplitude * sin_phases;
    
    (horizontal_x, horizontal_z, vertical_y)
}

/// Fast height-only query for Gerstner waves (for surfboard physics)
/// This skips horizontal displacement calculation when only height is needed
pub fn get_wave_height(position: Vec2, waves: &[WaveParameters], time: f32) -> f32 {
    let mut total_height = 0.0;
    
    for wave in waves {
        let dot_product = position.dot(wave.direction);
        let phase = wave.wave_number * dot_product - wave.speed * time;
        total_height += wave.amplitude * phase.sin();
    }
    
    total_height
}

/// Get wave height at a specific position (for use in other systems)
/// Call this from within your system that has access to time and water data
pub fn query_wave_height_at_time(position: Vec2, waves: &[WaveParameters], time: f32) -> f32 {
    get_wave_height(position, waves, time)
}

pub fn update_water_vertices(
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    query: Query<(&Mesh3d, &WaterSurface, &WaterWaves)>,
) {
    let elapsed = time.elapsed_secs();
    
    for (mesh_3d, surface, waves) in query.iter() {
        if let Some(mesh) = meshes.get_mut(&mesh_3d.0) {
            if let Some(positions) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
                if let bevy::render::mesh::VertexAttributeValues::Float32x3(pos_data) = positions {
                    // Process vertices in chunks of 4 for SIMD optimization
                    let base_positions = &surface.base_positions;
                    let vertex_count = base_positions.len();
                    
                    // Process SIMD chunks (4 vertices at a time)
                    for chunk_start in (0..vertex_count).step_by(4) {
                        let chunk_end = (chunk_start + 4).min(vertex_count);
                        
                        if chunk_end - chunk_start == 4 {
                            // Full SIMD chunk - process 4 vertices at once
                            let positions_x = f32x4::new([
                                base_positions[chunk_start].x,
                                base_positions[chunk_start + 1].x,
                                base_positions[chunk_start + 2].x,
                                base_positions[chunk_start + 3].x,
                            ]);
                            let positions_z = f32x4::new([
                                base_positions[chunk_start].z,
                                base_positions[chunk_start + 1].z,
                                base_positions[chunk_start + 2].z,
                                base_positions[chunk_start + 3].z,
                            ]);
                            
                            // Accumulate displacements from all waves
                            let mut total_dx = f32x4::splat(0.0);
                            let mut total_dz = f32x4::splat(0.0);
                            let mut total_dy = f32x4::splat(0.0);
                            
                            for wave in &waves.waves {
                                let (dx, dz, dy) = calculate_gerstner_displacement_simd(
                                    positions_x, positions_z, wave, elapsed
                                );
                                total_dx += dx;
                                total_dz += dz;
                                total_dy += dy;
                            }
                            
                            // Apply displacements to vertices
                            let dx_array: [f32; 4] = total_dx.to_array();
                            let dz_array: [f32; 4] = total_dz.to_array();
                            let dy_array: [f32; 4] = total_dy.to_array();
                            
                            for i in 0..4 {
                                let idx = chunk_start + i;
                                pos_data[idx][0] = base_positions[idx].x + dx_array[i];
                                pos_data[idx][1] = dy_array[i];
                                pos_data[idx][2] = base_positions[idx].z + dz_array[i];
                            }
                        } else {
                            // Handle remaining vertices with scalar calculation
                            for idx in chunk_start..chunk_end {
                                let base_pos = &base_positions[idx];
                                let pos_2d = Vec2::new(base_pos.x, base_pos.z);
                                
                                let mut total_displacement = (0.0f32, 0.0f32, 0.0f32);
                                for wave in &waves.waves {
                                    let (dx, dz, dy) = calculate_gerstner_displacement(pos_2d, wave, elapsed);
                                    total_displacement.0 += dx;
                                    total_displacement.1 += dz;
                                    total_displacement.2 += dy;
                                }
                                
                                pos_data[idx][0] = base_pos.x + total_displacement.0;
                                pos_data[idx][1] = total_displacement.2;
                                pos_data[idx][2] = base_pos.z + total_displacement.1;
                            }
                        }
                    }
                }
            }
            
            // Recompute normals for proper lighting with the new geometry
            mesh.compute_normals();
        }
    }
}

pub fn spawn_water(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let grid_size = 200;
    let world_size = 100.0;
    
    let (mesh, base_positions) = create_water_mesh(grid_size, world_size);
    let mesh_handle = meshes.add(mesh);
    
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.5, 0.8),
        perceptual_roughness: 0.3,
        metallic: 0.0,
        reflectance: 0.5,
        ..default()
    });
    
    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material),
        Transform::from_translation(Vec3::ZERO),
        WaterSurface {
            grid_size,
            world_size,
            vertex_count: grid_size * grid_size,
            base_positions,
        },
        WaterWaves::default(),
    ));
}

pub fn setup_camera(mut commands: Commands) {
    let isometric_angle = -26.565f32.to_radians();
    let rotation_y = 45f32.to_radians();
    let distance = 100.0;
    
    let rotation = Quat::from_euler(EulerRot::YXZ, rotation_y, isometric_angle, 0.0);
    let translation = rotation * Vec3::new(0.0, 0.0, distance);
    
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(translation).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    
    commands.spawn((
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -45f32.to_radians(), -45f32.to_radians(), 0.0)),
    ));
}

#[derive(Component, Debug)]
pub struct FloatingBody {
    pub buoyancy_points: Vec<Vec3>, // Relative positions from entity center to sample water height
    pub submerged_volume: f32,
    pub water_density: f32,
    pub body_density: f32,
    pub drag_coefficient: f32,
}

impl Default for FloatingBody {
    fn default() -> Self {
        Self {
            // Sample points for a surfboard - corners and center
            buoyancy_points: vec![
                Vec3::new(-1.5, 0.0, -0.3),  // Front left
                Vec3::new(1.5, 0.0, -0.3),   // Front right
                Vec3::new(-1.5, 0.0, 0.3),   // Back left
                Vec3::new(1.5, 0.0, 0.3),    // Back right
                Vec3::new(0.0, 0.0, 0.0),    // Center
            ],
            submerged_volume: 0.0,
            water_density: 1000.0,   // kg/m³
            body_density: 200.0,     // Surfboard is much lighter than water
            drag_coefficient: 0.1,
        }
    }
}

#[derive(Component, Debug)]
pub struct Surfboard {
    pub length: f32,
    pub width: f32,
    pub thickness: f32,
}

impl Default for Surfboard {
    fn default() -> Self {
        Self {
            length: 3.0,   // 3 meter surfboard
            width: 0.6,    // 60cm wide
            thickness: 0.1, // 10cm thick
        }
    }
}

pub fn create_surfboard_mesh(surfboard: &Surfboard) -> Mesh {
    let half_length = surfboard.length / 2.0;
    let half_width = surfboard.width / 2.0;
    let half_thickness = surfboard.thickness / 2.0;
    
    // Simple box mesh for the surfboard
    let positions = vec![
        // Bottom face
        [-half_length, -half_thickness, -half_width],
        [half_length, -half_thickness, -half_width],
        [half_length, -half_thickness, half_width],
        [-half_length, -half_thickness, half_width],
        // Top face
        [-half_length, half_thickness, -half_width],
        [half_length, half_thickness, -half_width],
        [half_length, half_thickness, half_width],
        [-half_length, half_thickness, half_width],
    ];
    
    let normals = vec![
        // Bottom face
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        // Top face
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    
    let uvs = vec![
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], // Bottom
        [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], // Top
    ];
    
    let indices = vec![
        // Bottom face
        0, 1, 2, 2, 3, 0,
        // Top face
        4, 6, 5, 6, 4, 7,
        // Side faces
        0, 4, 5, 5, 1, 0,
        1, 5, 6, 6, 2, 1,
        2, 6, 7, 7, 3, 2,
        3, 7, 4, 4, 0, 3,
    ];
    
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    
    mesh
}

pub fn spawn_surfboard(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let surfboard = Surfboard::default();
    let mesh = create_surfboard_mesh(&surfboard);
    let mesh_handle = meshes.add(mesh);
    
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 1.0, 0.8), // Off-white surfboard color
        perceptual_roughness: 0.8,
        metallic: 0.0,
        ..default()
    });
    
    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material),
        Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)), // Start above water
        surfboard,
        FloatingBody::default(),
    ));
}

pub fn update_surfboard_physics(
    time: Res<Time>,
    water_query: Query<&WaterWaves>,
    mut surfboard_query: Query<(&mut Transform, &mut FloatingBody, &Surfboard)>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();
    
    if let Ok(waves) = water_query.single() {
        for (mut transform, mut floating_body, _surfboard) in surfboard_query.iter_mut() {
            let position = transform.translation;
            
            // Sample water height at buoyancy points
            let mut total_buoyancy_force = 0.0;
            let mut total_torque = Vec3::ZERO;
            let mut submerged_points = 0;
            
            for buoyancy_point in &floating_body.buoyancy_points {
                // Transform buoyancy point to world space
                let world_point = position + transform.rotation * *buoyancy_point;
                let sample_pos = Vec2::new(world_point.x, world_point.z);
                
                // Get water height at this point
                let water_height = get_wave_height(sample_pos, &waves.waves, elapsed);
                
                // Calculate how much this point is submerged
                let submersion = water_height - world_point.y;
                
                if submersion > 0.0 {
                    submerged_points += 1;
                    
                    // Apply buoyancy force (Archimedes principle)
                    let buoyancy_force = floating_body.water_density * 9.81 * submersion.min(0.2); // Cap submersion
                    total_buoyancy_force += buoyancy_force;
                    
                    // Calculate torque for tilting
                    let force_point = *buoyancy_point;
                    let force_vector = Vec3::new(0.0, buoyancy_force, 0.0);
                    total_torque += force_point.cross(force_vector);
                }
            }
            
            // Update submerged volume for reference
            floating_body.submerged_volume = submerged_points as f32 / floating_body.buoyancy_points.len() as f32;
            
            // Apply forces
            let gravity = -9.81 * floating_body.body_density;
            let net_vertical_force = total_buoyancy_force + gravity;
            
            // Simple physics integration
            let acceleration = net_vertical_force / floating_body.body_density;
            transform.translation.y += acceleration * dt * dt;
            
            // Apply drag to prevent excessive bouncing
            transform.translation.y *= 1.0 - floating_body.drag_coefficient * dt;
            
            // Apply gentle rotation based on wave slope (simplified)
            if total_torque.length() > 0.01 {
                let rotation_speed = total_torque * 0.1 * dt;
                let rotation = Quat::from_euler(EulerRot::XYZ, rotation_speed.x, 0.0, rotation_speed.z);
                transform.rotation = (transform.rotation * rotation).normalize();
            }
            
            // Damp rotation to prevent excessive spinning
            transform.rotation = transform.rotation.slerp(Quat::IDENTITY, floating_body.drag_coefficient * dt);
        }
    }
}

pub struct WaterPlugin;

impl Plugin for WaterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
            .add_plugins(bevy::diagnostic::LogDiagnosticsPlugin::default())
            .add_systems(Startup, (spawn_water, setup_camera, spawn_surfboard))
            .add_systems(FixedUpdate, (update_water_vertices, update_surfboard_physics));
    }
}