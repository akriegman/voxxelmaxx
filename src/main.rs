use avian3d::prelude::*;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PhysicsPlugins::default())
        .add_systems(Startup, setup)
        .add_systems(Update, proc_gen)
        .add_systems(FixedUpdate, clean_chunks)
        .add_systems(Update, movement)
        .run();
}

fn setup(mut cmd: Commands) {
    // player
    cmd.spawn((Camera3D::default(), Transform::IDENTITY));

    // world
    cmd.spawn((Body {}, ProcGen { seed: Vec3::ZERO }));
}

#[derive(Component)]
struct Body {
    chunks: HashMap<IVec3, Entity>,
}

#[derive(Bundle)]
struct Chunk<N> {
    tags: Box<[u8; N * N * N]>,
    collider: Collider,
    transform: Transform,
    mesh: Mesh3d,
    material: MeshMaterial3d,
}

struct ProcGen {
    seed: Vec3,
}

impl Chunk<N> {
    // CLAUDE so I guess to create a chunk we need access to global resources?
    // we're gonna be creating chunks in lots of places, always with an empty mesh...
    // or I guess instead of adding the mesh here, we can just add it in update_chunks
    // as long as we either overwrite the old mesh or it knows to deallocate it somehow
    fn new(idx: IVec3, mesh: Mesh3d, material: MeshMaterial3d) -> Self {
        Self {
            tags: Box::new([0; N * N * N]),
            collider: Collider::voxels(Vec3::splat(1. / N), &[]),
            transform: Transform::from_translation(idx.as_vec3()),
            mesh,
            material,
        }
    }
}

// CLAUDE finish this
fn proc_gen(mut commands: Commands /* player, world */) {
    // for coords: Ivec3 within some radius of the player:
    //     if there is no chunk at those coords:
    //         generate the chunk
    // use spherical radius
    // generate the voxels by calculating two noise functions u v, then using u, v+y to sample from assets/gen_map.png, and taking the red channel
    // spawn the chunks as children entities of the world
}

/// rebuilds colliders and meshes
// TODO should we separate this so that colliders can be on FixedUpdate and meshes on Update?
fn clean_chunks(/* chunks */) {
    // CLAUDE tell the voxel colliders to recompute if needed
    // including prodding neighboring chunks if needed. they have methods for that.
    // then build the mesh from the collider, they have methods for that too.
    // I guess this will require some dirty flagging system
}

// CLAUDE can you implement standard FPS movement please? and use shift to go down and space to go up. no gravity for now.
fn movement() {}
