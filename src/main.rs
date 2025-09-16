use std::time::Instant;

use bevy::{
    asset::RenderAssetUsages,
    color::palettes::css::MAGENTA,
    core_pipeline::tonemapping::Tonemapping,
    image::TextureFormatPixelInfo,
    mesh::PlaneMeshBuilder,
    prelude::*,
    render::render_resource::*,
    window::{PrimaryWindow, WindowResized},
};
use glaciers::{GlaciersContext, GlaciersPlugin};

pub const BLACK: Srgba = Srgba::rgb(0.0, 0.0, 0.0);
pub const WHITE: Srgba = Srgba::rgb(1.0, 1.0, 1.0);

pub const RED: Srgba = Srgba::rgb(1.0, 0.0, 0.0);
pub const GREEN: Srgba = Srgba::rgb(0.0, 1.0, 0.0);
pub const BLUE: Srgba = Srgba::rgb(0.0, 0.0, 1.0);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, GlaciersPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (rotate, handle_resize, handle_input, draw))
        .run();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let res = &window.single().unwrap().resolution;
    let scale = 1.0;
    let image = Image::new_fill(
        Extent3d {
            width: (res.width() * scale) as u32,
            height: (res.height() * scale) as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8; 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );

    let half_width = image.size_f32().x / 2.0;
    let half_height = image.size_f32().y / 2.0;

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Tonemapping::None,
        GlaciersContext {
            image: images.add(image),
            scale,
            ..default()
        },
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

#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub pos: Vec3,
    pub color: LinearRgba,
}

impl Vertex {
    pub fn new(pos: Vec3, color: Color) -> Self {
        Self {
            pos: pos,
            color: color.into(),
        }
    }
}

#[derive(Component)]
struct Triangle {
    vertices: [Vertex; 3],
    aabb: (Vec3, Vec3),
}

impl Triangle {
    fn new(vertices: [Vertex; 3]) -> Self {
        let aabb = vertices.iter().fold(
            (vertices[0].pos, vertices[0].pos),
            |(prev_min, prev_max), point| (point.pos.min(prev_min), point.pos.max(prev_max)),
        );
        Self { vertices, aabb }
    }
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

fn handle_resize(
    mut ctx: Query<&GlaciersContext>,
    mut images: ResMut<Assets<Image>>,
    mut resize_events: MessageReader<WindowResized>,
) {
    let Ok(ctx) = ctx.single_mut() else {
        return;
    };
    let Some(image) = images.get_mut(ctx.image.id()) else {
        return;
    };

    for ev in resize_events.read() {
        if image.size_f32().x != ev.width * ctx.scale || image.size_f32().y != ev.height * ctx.scale
        {
            image.resize(Extent3d {
                width: (ev.width * ctx.scale) as u32,
                height: (ev.height * ctx.scale) as u32,
                depth_or_array_layers: 1,
            });
            println!("Image size: {} ", image.size());
        }
    }
}

fn draw(
    mut ctx: Query<&GlaciersContext>,
    mut images: ResMut<Assets<Image>>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
    meshes: Query<(&Mesh3d, &GlobalTransform)>,
    meshes_assets: Res<Assets<Mesh>>,
    views: Query<(&Camera, &GlobalTransform)>,
) -> Result<()> {
    let Ok(ctx) = ctx.single_mut() else {
        return Ok(());
    };
    let Some(image) = images.get_mut(ctx.image.id()) else {
        return Ok(());
    };
    let Ok((camera, global_camera)) = views.single() else {
        return Ok(());
    };

    let start = Instant::now();

    // Clear the image
    if let Some(data) = image.data.as_mut() {
        for old_pixel in data.chunks_mut(image.texture_descriptor.format.pixel_size().unwrap()) {
            old_pixel.copy_from_slice(&[0; 4]);
        }
    }

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
                    let view_pos = view_pos * ctx.scale;
                    if view_pos.y < 0.0
                        || view_pos.y > image.size_f32().y
                        || view_pos.x < 0.0
                        || view_pos.x > image.size_f32().x
                    {
                        continue 'outer;
                    }
                    vertices[i] = Vertex::new(view_pos, color);
                }
                let triangle = Triangle::new(vertices);
                draw_triangle(image, &triangle);
                draw_triangle_wireframe(image, &triangle, BLACK.into());

                primitive_id += 1;
            }
        } else {
            for x in pos.chunks(3) {
                let &[pos0, pos1, pos2] = x else {
                    unreachable!()
                };
                let tri = Triangle::new([
                    Vertex::new(pos0.into(), RED.into()),
                    Vertex::new(pos1.into(), RED.into()),
                    Vertex::new(pos2.into(), RED.into()),
                ]);
                draw_triangle(image, &tri);
                draw_triangle_wireframe(image, &tri, BLACK.into());
            }
        }
    }

    let frame_time = start.elapsed().as_secs_f32() * 1000.0;
    let fps = 1000.0 / frame_time;

    window.single_mut().unwrap().title = format!(
        "Glaciers - {}x{} {:.2}ms {:.0}fps",
        image.size().x,
        image.size().y,
        frame_time,
        fps
    );

    Ok(())
}

fn edge_function(a: IVec2, b: IVec2, c: IVec2) -> i32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn draw_triangle(image: &mut Image, triangle: &Triangle) {
    let Triangle {
        vertices,
        aabb: (min, max),
    } = triangle;

    // Only check the pixels inside the AABB
    for x in min.x as u32..=max.x as u32 {
        for y in min.y as u32..=max.y as u32 {
            let a = vertices[0].pos.xy().as_ivec2();
            let b = vertices[1].pos.xy().as_ivec2();
            let c = vertices[2].pos.xy().as_ivec2();

            let p = UVec2::new(x, y).as_ivec2();

            let abp = edge_function(a, b, p);
            let bcp = edge_function(b, c, p);
            let cap = edge_function(c, a, p);

            let abc = edge_function(a, b, c);

            let weight_a = bcp as f32 / abc as f32;
            let weight_b = cap as f32 / abc as f32;
            let weight_c = abp as f32 / abc as f32;

            // Normally you only need to check one of these, but I don't know the winding order of
            // the triangle
            if (abp >= 0 && bcp >= 0 && cap >= 0) || (abp <= 0 && bcp <= 0 && cap <= 0) {
                let weights = Vec3::new(weight_a, weight_b, weight_c);
                let color = Mat3::from_cols(
                    vertices[0].color.to_vec3(),
                    vertices[1].color.to_vec3(),
                    vertices[2].color.to_vec3(),
                ) * weights;
                let alpha = Vec3::new(
                    vertices[0].color.alpha(),
                    vertices[1].color.alpha(),
                    vertices[2].color.alpha(),
                )
                .dot(weights);

                draw_point(
                    image,
                    p.as_uvec2(),
                    Color::srgba(color.x, color.y, color.z, alpha),
                );
            }
        }
    }
}

fn draw_triangle_wireframe(image: &mut Image, Triangle { vertices, .. }: &Triangle, color: Color) {
    draw_line(image, vertices[0].pos, vertices[1].pos, color);
    draw_line(image, vertices[1].pos, vertices[2].pos, color);
    draw_line(image, vertices[2].pos, vertices[0].pos, color);
}

fn draw_line(image: &mut Image, start: Vec3, end: Vec3, color: Color) {
    let mut x0 = start.x as i32;
    let mut y0 = start.y as i32;
    let x1 = end.x as i32;
    let y1 = end.y as i32;

    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        draw_point(image, UVec2::new(x0 as u32, y0 as u32), color);

        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn draw_point(image: &mut Image, pos: UVec2, color: Color) {
    let _ = image.set_color_at(pos.x, pos.y, color);
}

#[derive(Component)]
struct Rotates;

/// Rotates any entity around the x and y axis
fn rotate(time: Res<Time>, mut query: Query<&mut Transform, With<Rotates>>) {
    let speed = 1.5;
    for mut transform in &mut query {
        transform.rotate_x(0.55 * time.delta_secs() * speed);
        transform.rotate_z(0.15 * time.delta_secs() * speed);
    }
}
