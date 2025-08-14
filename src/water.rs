use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
    },
};

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
}

#[derive(Component, Debug)]
pub struct WaterWaves {
    pub waves: Vec<WaveParameters>,
}

impl Default for WaterWaves {
    fn default() -> Self {
        Self {
            waves: vec![
                WaveParameters {
                    amplitude: 1.0,
                    wavelength: 20.0,
                    speed: 2.0,
                    direction: Vec2::new(1.0, 0.3).normalize(),
                },
                WaveParameters {
                    amplitude: 0.6,
                    wavelength: 15.0,
                    speed: 1.8,
                    direction: Vec2::new(0.8, 0.2).normalize(),
                },
                WaveParameters {
                    amplitude: 0.4,
                    wavelength: 12.0,
                    speed: 2.2,
                    direction: Vec2::new(1.2, -0.1).normalize(),
                },
                WaveParameters {
                    amplitude: 0.3,
                    wavelength: 8.0,
                    speed: 2.5,
                    direction: Vec2::new(0.9, 0.4).normalize(),
                },
            ],
        }
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

fn calculate_wave_height(position: Vec2, wave: &WaveParameters, time: f32) -> f32 {
    let k = 2.0 * std::f32::consts::PI / wave.wavelength;
    let dot_product = position.dot(wave.direction);
    let phase = k * dot_product - wave.speed * time;
    wave.amplitude * phase.sin()
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
                    for (idx, base_pos) in surface.base_positions.iter().enumerate() {
                        let pos_2d = Vec2::new(base_pos.x, base_pos.z);
                        
                        let mut height = 0.0;
                        for wave in &waves.waves {
                            height += calculate_wave_height(pos_2d, wave, elapsed);
                        }
                        
                        pos_data[idx][1] = height;
                    }
                }
            }
            
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

pub struct WaterPlugin;

impl Plugin for WaterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
            .add_plugins(bevy::diagnostic::LogDiagnosticsPlugin::default())
            .add_systems(Startup, (spawn_water, setup_camera))
            .add_systems(FixedUpdate, update_water_vertices);
    }
}