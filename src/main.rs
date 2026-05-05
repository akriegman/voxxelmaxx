use std::collections::HashMap;

use avian3d::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    input::mouse::AccumulatedMouseMotion,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use noisy_bevy::simplex_noise_3d_seeded;

const N: usize = 16;
const N_F: f32 = N as f32;
const VOXEL_SIZE: f32 = 1.0 / N_F;
const LOAD_RADIUS: i32 = 8;
const MAX_NEW_CHUNKS_PER_FRAME: usize = usize::MAX;
const NOISE_SCALE: f32 = 0.02;
const MOUSE_SENS: f32 = 0.002;
const MOVE_FORCE: f32 = 4.0;
const JUMP_IMPULSE: f32 = 1.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PhysicsPlugins::default())
        .insert_resource(Gravity::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (proc_gen, movement))
        .add_systems(FixedUpdate, clean_chunks)
        .run();
}

fn setup(
    mut cmd: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    // player
    cmd.spawn((
        Player::default(),
        Transform::from_xyz(0., 16., 0.),
        Visibility::default(),
        RigidBody::Dynamic,
        Collider::capsule(0.25, 0.5),
        Mesh3d(meshes.add(Capsule3d::new(0.25, 0.5))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        LockedAxes::ROTATION_LOCKED,
        Friction::new(0.1),
        LinearDamping(2.0),
    ))
    .with_child((Camera3d::default(), Transform::from_xyz(0., 0.25, 0.)));

    // world
    cmd.spawn((
        Body::default(),
        RigidBody::Static,
        ProcGen {
            seed: Vec3::ZERO,
            image: asset_server.load("gen_map.png"),
        },
    ));

    // light
    cmd.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(2., 4., 1.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    cmd.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(-2., 4., -1.).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    cmd.insert_resource(TerrainMaterial(materials.add(Color::srgb(0.45, 0.6, 0.35))));

    cursor_options.grab_mode = CursorGrabMode::Locked;
    cursor_options.visible = false;
}

#[derive(Component, Default)]
struct Player {
    yaw: f32,
    pitch: f32,
}

#[derive(Component, Default)]
#[require(Transform, Visibility)]
struct Body {
    chunks: HashMap<IVec3, Entity>,
}

#[derive(Component)]
struct Chunk(Box<[u8; N * N * N]>);

#[derive(Component)]
struct ProcGen {
    seed: Vec3,
    image: Handle<Image>,
}

#[derive(Resource)]
struct TerrainMaterial(Handle<StandardMaterial>);

impl Body {
    /// Spawn a new chunk entity at `idx`, register it in this body's index,
    /// and parent it to `body_entity`. Returns the new chunk entity.
    fn add_chunk(
        &mut self,
        commands: &mut Commands,
        body_entity: Entity,
        idx: IVec3,
        tags: Box<[u8; N * N * N]>,
    ) -> Entity {
        let e = commands
            .spawn((Chunk(tags), Transform::from_translation(idx.as_vec3())))
            .id();
        commands.entity(body_entity).add_child(e);
        self.chunks.insert(idx, e);
        e
    }

    // TODO setter for in-place voxel edits: mutate the target chunk's tags
    // and mut-deref each neighboring chunk (without changing data) so their
    // Changed<Chunk> fires too — keeps collider seams continuous via
    // Voxels::combine_voxel_states in clean_chunks.
}

fn proc_gen(
    mut commands: Commands,
    mut bodies: Query<(Entity, &mut Body, &ProcGen)>,
    player: Single<&Transform, With<Player>>,
    images: Res<Assets<Image>>,
) {
    let player_chunk = player.translation.floor().as_ivec3();
    let r = LOAD_RADIUS;
    let r_sq = (r * r) as f32;

    let mut spawned = 0;

    for (body_e, mut body, proc) in &mut bodies {
        let Some(map) = images.get(&proc.image) else {
            continue;
        };
        let (w, h) = (map.width() as i32, map.height() as i32);
        let data = map.data.as_deref().unwrap_or(&[]);
        let bytes_per_pixel = (w * h)
            .try_into()
            .ok()
            .and_then(|n: usize| data.len().checked_div(n))
            .unwrap_or(4);
        let seed_u = proc.seed + Vec3::splat(101.0);
        let seed_v = proc.seed + Vec3::splat(307.0);

        for dx in -r..=r {
            for dy in -r..=r {
                for dz in -r..=r {
                    if spawned >= MAX_NEW_CHUNKS_PER_FRAME {
                        return;
                    }
                    let offset = IVec3::new(dx, dy, dz);
                    if offset.as_vec3().length_squared() > r_sq {
                        continue;
                    }
                    let idx = player_chunk + offset;
                    if body.chunks.contains_key(&idx) {
                        continue;
                    }

                    let mut tags = Box::new([0u8; N * N * N]);
                    let chunk_origin = idx * N as i32;
                    for lz in 0..N {
                        for ly in 0..N {
                            for lx in 0..N {
                                let local = IVec3::new(lx as i32, ly as i32, lz as i32);
                                let p = (chunk_origin + local).as_vec3() * NOISE_SCALE;
                                let u = simplex_noise_3d_seeded(p, seed_u) * 0.5 + 0.5;
                                let v = simplex_noise_3d_seeded(p, seed_v) * 0.5 + 0.5;
                                // sample (u, v + y): u,v vary smoothly with world pos,
                                // and y advances vertically through the gen_map strip.
                                let sx = (u * w as f32) as i32;
                                let sy = (v * h as f32 + p.y * 4.) as i32;
                                let sx = if sx == w { sx - 1 } else { sx };
                                assert!(0 <= sx && sx < w, "{sx} not in range 0..{w}");
                                tags[lx + N * (ly + N * lz)] = if 0 <= sy && sy < h {
                                    let pixel_idx = ((sy * w + sx) as usize) * bytes_per_pixel;
                                    data.get(pixel_idx).copied().unwrap()
                                } else {
                                    0
                                }
                            }
                        }
                    }

                    body.add_chunk(&mut commands, body_e, idx, tags);
                    spawned += 1;
                }
            }
        }
    }
}

/// rebuilds colliders and meshes
// TODO should we separate this so that colliders can be on FixedUpdate and meshes on Update?
// TODO once we have >1 material, split each chunk into one sub-entity per
// contiguous same-material group, each with its own collider+mesh+material.
// Then rename Body -> Grid, and the per-material sub-entities become Bodies.
fn clean_chunks(
    mut commands: Commands,
    chunks: Query<(Entity, &Chunk), Changed<Chunk>>,
    mut meshes: ResMut<Assets<Mesh>>,
    terrain_mat: Res<TerrainMaterial>,
) {
    for (entity, chunk) in &chunks {
        let mut coords: Vec<IVec3> = Vec::new();
        for lz in 0..N {
            for ly in 0..N {
                for lx in 0..N {
                    if chunk.0[lx + N * (ly + N * lz)] != 0 {
                        coords.push(IVec3::new(lx as i32, ly as i32, lz as i32));
                    }
                }
            }
        }

        // Avian panics on an empty voxel collider, and an empty chunk has nothing
        // to render either — strip any previously-attached collider/mesh/material.
        if coords.is_empty() {
            commands
                .entity(entity)
                .remove::<(Collider, Mesh3d, MeshMaterial3d<StandardMaterial>)>();
            continue;
        }

        let collider = Collider::voxels(Vec3::splat(VOXEL_SIZE), &coords);

        let (vertices, indices) = collider
            .shape()
            .as_voxels()
            .expect("just built voxel collider")
            .to_trimesh();
        let vertices: Vec<[f32; 3]> = vertices
            .iter()
            .map(|v| [v.x as f32, v.y as f32, v.z as f32])
            .collect();
        let indices: Vec<u32> = indices.into_iter().flatten().collect();
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_duplicated_vertices()
        .with_computed_flat_normals();

        // TODO once the setter exists, call Voxels::combine_voxel_states with
        // each present neighbor here for seamless collision across seams.
        commands.entity(entity).insert((
            collider,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(terrain_mat.0.clone()),
        ));
    }
}

// CLAUDE can you implement standard FPS movement please? and use shift to go down and space to go up. no gravity for now.
fn movement(
    mut player: Single<(Forces, &mut Player, &mut Transform), With<Player>>,
    mut cam: Single<&mut Transform, (With<Camera3d>, Without<Player>)>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<AccumulatedMouseMotion>,
) {
    let (forces, player, body_tf) = &mut *player;

    if mouse.delta != Vec2::ZERO {
        player.yaw -= mouse.delta.x * MOUSE_SENS;
        player.pitch = (player.pitch - mouse.delta.y * MOUSE_SENS).clamp(-1.54, 1.54);
        body_tf.rotation = Quat::from_axis_angle(Vec3::Y, player.yaw);
        cam.rotation = Quat::from_axis_angle(Vec3::X, player.pitch);
    }

    let forward = body_tf.forward();
    let forward_h = Vec3::new(forward.x, 0., forward.z).normalize_or_zero();
    let right = body_tf.right();
    let right_h = Vec3::new(right.x, 0., right.z).normalize_or_zero();

    let mut dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        dir += forward_h;
    }
    if keys.pressed(KeyCode::KeyS) {
        dir -= forward_h;
    }
    if keys.pressed(KeyCode::KeyD) {
        dir += right_h;
    }
    if keys.pressed(KeyCode::KeyA) {
        dir -= right_h;
    }

    forces.apply_force(dir.normalize_or_zero() * MOVE_FORCE);

    if keys.just_pressed(KeyCode::Space) {
        forces.apply_linear_impulse(Vec3::Y * JUMP_IMPULSE);
    }
}
