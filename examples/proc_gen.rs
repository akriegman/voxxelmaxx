//! This example showcases a few things because it was the first
//! example. Procedural generation, a first person dynamic grid
//! player, and breaking / placing voxels. Uses Minecraft controls.

use avian3d::prelude::*;
use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow, Window, WindowMode},
};
use noisy_bevy::fbm_simplex_3d_seeded;

use voxxelmaxx::{Grid, H, N, TerrainMaterial, VoxelPlugin};

const LOAD_RADIUS: i32 = 8;
const MAX_NEW_CHUNKS_PER_FRAME: usize = 12;
const MOUSE_SENS: f32 = 0.002;
const MOVE_FORCE: f32 = 4.0;
const JUMP_IMPULSE: f32 = 1.0;
const PLACE_TAG: u8 = 0x80;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(VoxelPlugin)
        .insert_resource(Gravity::default())
        .add_systems(Startup, setup)
        .add_systems(Update, movement)
        .add_systems(Update, build_break)
        .add_systems(Update, proc_gen)
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
        Transform::from_xyz(0., 4., 0.),
        Visibility::default(),
        RigidBody::Dynamic,
        Collider::cylinder(0.25, 1.),
        Mesh3d(meshes.add(Capsule3d::new(0.25, 0.5))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        LockedAxes::ROTATION_LOCKED,
        Friction::new(0.1),
        LinearDamping(2.0),
    ))
    .with_child((Camera3d::default(), Transform::from_xyz(0., 0.25, 0.)));

    // world
    cmd.spawn((
        Grid::default(),
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

    cmd.insert_resource(TerrainMaterial(materials.add(Color::WHITE)));

    cursor_options.grab_mode = CursorGrabMode::Locked;
    cursor_options.visible = false;
}

#[derive(Component, Default)]
struct Player {
    yaw: f32,
    pitch: f32,
}

#[derive(Component)]
pub struct ProcGen {
    seed: Vec3,
    image: Handle<Image>,
}

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

fn build_break(
    mouse: Res<ButtonInput<MouseButton>>,
    spatial: SpatialQuery,
    cam: Single<&GlobalTransform, With<Camera3d>>,
    player_e: Single<Entity, With<Player>>,
    parents: Query<&ChildOf>,
    mut grids: Query<&mut Grid>,
) {
    let break_ = mouse.pressed(MouseButton::Left);
    let place = mouse.pressed(MouseButton::Right);
    if !(break_ ^ place) {
        return;
    }

    let origin = cam.translation();
    let dir = cam.forward();
    let filter = SpatialQueryFilter::from_excluded_entities([*player_e]);
    let Some(hit) = spatial.cast_ray(origin, dir, f32::INFINITY, true, &filter) else {
        return;
    };
    // Hit lands on a chunk render-child entity; the grid is its parent.
    let Ok(child_of) = parents.get(hit.entity) else {
        return;
    };
    let Ok(mut grid) = grids.get_mut(child_of.parent()) else {
        return;
    };

    let (vox, face) = grid.rayhit_face(origin, *dir, &hit);
    let target = if break_ { vox } else { vox + face };
    let tag = if break_ { 0 } else { PLACE_TAG };
    grid.set(target, tag);
}

fn proc_gen(
    mut grids: Query<(&mut Grid, &ProcGen)>,
    player: Single<&Transform, With<Player>>,
    images: Res<Assets<Image>>,
) {
    let player_chunk = player.translation.floor().as_ivec3();
    let r = LOAD_RADIUS;
    let r_sq = (r * r) as f32;

    let mut spawned = 0;

    for (mut grid, proc) in &mut grids {
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
                    let added = grid.add_chunk(idx, || {
                        let mut tags = Box::new([0u8; N * N * N]);
                        for lz in 0..N {
                            for ly in 0..N {
                                for lx in 0..N {
                                    let local = Vec3::new(lx as f32, ly as f32, lz as f32);
                                    let p = idx.as_vec3() + local * H;
                                    let u =
                                        fbm_simplex_3d_seeded(p / w as f32, 5, 2.0, 0.5, seed_u)
                                            * 0.5
                                            + 0.5;
                                    let v =
                                        fbm_simplex_3d_seeded(p / h as f32, 5, 2.0, 0.5, seed_v)
                                            * 0.25;
                                    // sample (u, v + y): u,v vary smoothly with world pos,
                                    // and y advances vertically through the gen_map strip.
                                    let sx = u as i32;
                                    let sy = (v as f32 - p.y / h as f32) as i32;
                                    let sx = sx.clamp(0, w - 1);
                                    tags[lx + N * (ly + N * lz)] = if 0 <= sy && sy < h {
                                        let pixel_idx = ((sy * w + sx) as usize) * bytes_per_pixel;
                                        data.get(pixel_idx).copied().unwrap()
                                    } else {
                                        0
                                    }
                                }
                            }
                        }
                        tags
                    });
                    if added {
                        spawned += 1;
                    }
                }
            }
        }
    }
}
