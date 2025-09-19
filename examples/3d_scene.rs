use std::time::Instant;

use bevy::{
    color::palettes::css::MAGENTA, core_pipeline::tonemapping::Tonemapping, mesh::PlaneMeshBuilder,
    prelude::*, window::PrimaryWindow,
};
use glaciers::{
    GlaciersParams,
    canvas::{Triangle, Vertex},
    plugin::GlaciersPlugin,
};

pub const BLACK: Srgba = Srgba::rgb(0.0, 0.0, 0.0);
pub const WHITE: Srgba = Srgba::rgb(1.0, 1.0, 1.0);

pub const RED: Srgba = Srgba::rgb(1.0, 0.0, 0.0);
pub const GREEN: Srgba = Srgba::rgb(0.0, 1.0, 0.0);
pub const BLUE: Srgba = Srgba::rgb(0.0, 0.0, 1.0);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, GlaciersPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (rotate, handle_input, draw))
        .run();
}

fn setup(
    mut commands: Commands,
    mut glaciers_params: GlaciersParams,
    window: Query<&Window, With<PrimaryWindow>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let scale = 1.0;
    let res = window.single().unwrap().resolution.clone();
    let glaciers_context = glaciers_params.init_context(res, scale);

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Tonemapping::None,
        glaciers_context,
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        Transform::from_xyz(0.0, 1.0, 0.0),
        Rotates,
    ));

    commands.spawn((
        Mesh3d(
            meshes.add(
                PlaneMeshBuilder::new(Dir3::Y, Vec2::splat(3.0))
                    .subdivisions(0)
                    .build(),
            ),
        ),
        Transform::default(),
    ));
}

fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut camera: Query<&mut Transform, With<Camera>>,
    time: Res<Time>,
) {
    // Exit
    if keyboard.just_pressed(KeyCode::Escape) {
        std::process::exit(1);
    }

    // Camera controller
    let speed = 5.0;
    let rotation_speed = speed * 2.0;
    for mut transform in &mut camera {
        let forward: Vec3 = transform.forward().into();
        let left: Vec3 = transform.left().into();
        let up: Vec3 = transform.up().into();
        if keyboard.pressed(KeyCode::KeyW) {
            transform.translation += forward * time.delta_secs() * speed;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            transform.translation -= forward * time.delta_secs() * speed;
        }
        if keyboard.pressed(KeyCode::KeyA) {
            transform.translation += left * time.delta_secs() * rotation_speed;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            transform.translation -= left * time.delta_secs() * rotation_speed;
        }
        if keyboard.pressed(KeyCode::KeyQ) {
            transform.translation -= up * time.delta_secs() * rotation_speed;
        };
        if keyboard.pressed(KeyCode::KeyE) {
            transform.translation += up * time.delta_secs() * rotation_speed;
        };

        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

fn draw(
    mut glaciers_params: GlaciersParams,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
    meshes: Query<(&Mesh3d, &GlobalTransform)>,
    meshes_assets: Res<Assets<Mesh>>,
    views: Query<(&Camera, &GlobalTransform)>,
) -> Result<()> {
    let scale = glaciers_params.context().scale;
    let mut canvas = glaciers_params.canvas();

    let Ok((camera, global_camera)) = views.single() else {
        return Ok(());
    };

    let start = Instant::now();

    canvas.clear();

    for (mesh_3d, transform) in &meshes {
        let Some(mesh) = meshes_assets.get(mesh_3d.id()) else {
            warn!("Missing mesh asset");
            continue;
        };
        let Some(pos) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
            warn!("Missing vertex attribute position");
            continue;
        };
        let Some(pos) = pos.as_float3() else {
            warn!("Failed to convert pos to float3");
            continue;
        };
        if let Some(indices) = mesh.indices() {
            let mut primitive_id = 0;

            let mut iter = indices.iter().peekable();
            'outer: while iter.peek().is_some() {
                let tri_indices = [
                    iter.next().unwrap(),
                    iter.next().unwrap(),
                    iter.next().unwrap(),
                ];
                fastrand::seed(primitive_id);
                let color = Color::srgba(fastrand::f32(), fastrand::f32(), fastrand::f32(), 1.0);
                let mut vertices = [Vertex::new(Vec3::ZERO, MAGENTA.into()); 3];
                for (i, &tri_i) in tri_indices.iter().enumerate() {
                    let pos: Vec3 = pos[tri_i].into();
                    let pos = transform.transform_point(pos);
                    let view_pos = match camera.world_to_viewport_with_depth(global_camera, pos) {
                        Ok(view_pos) => view_pos,
                        Err(err) => {
                            warn!("Triangle needs to be clipped. {err:?}");
                            continue 'outer;
                        }
                    };
                    let view_pos = view_pos * scale;
                    if view_pos.y < 0.0
                        || view_pos.y > canvas.size_f32().y
                        || view_pos.x < 0.0
                        || view_pos.x > canvas.size_f32().x
                    {
                        continue 'outer;
                    }
                    vertices[i] = Vertex::new(view_pos, color);
                }
                let triangle = Triangle::new(vertices);
                canvas.draw_triangle_wide(&triangle);
                canvas.draw_triangle_wireframe(&triangle, BLACK.to_u8_array());

                primitive_id += 1;
            }
        } else {
            for x in pos.chunks(3) {
                let &[_pos0, _pos1, _pos2] = x else {
                    unreachable!()
                };
                unimplemented!()
            }
        }
    }

    let frame_time = start.elapsed().as_secs_f32() * 1000.0;
    let fps = 1000.0 / frame_time;

    window.single_mut().unwrap().title = format!(
        "Glaciers - {}x{} {:.2}ms {:.0}fps",
        canvas.size().x,
        canvas.size().y,
        frame_time,
        fps
    );

    Ok(())
}

#[derive(Component)]
struct Rotates;

/// Rotates any entity around the x and z axis
fn rotate(time: Res<Time>, mut query: Query<&mut Transform, With<Rotates>>) {
    let speed = 1.5;
    for mut transform in &mut query {
        transform.rotate_x(0.55 * time.delta_secs() * speed);
        transform.rotate_z(0.15 * time.delta_secs() * speed);
    }
}
