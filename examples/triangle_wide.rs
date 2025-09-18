use std::time::{Duration, Instant};

use bevy::{
    asset::RenderAssetUsages, core_pipeline::tonemapping::Tonemapping, prelude::*,
    render::render_resource::*, window::PrimaryWindow,
};
use glaciers::{
    GlaciersContext,
    canvas::{Triangle, Vertex},
    plugin::GlaciersPlugin,
};

pub const BLACK: Srgba = Srgba::rgb(0.0, 0.0, 0.0);
pub const WHITE: Srgba = Srgba::rgb(1.0, 1.0, 1.0);

pub const RED: Srgba = Srgba::rgb(1.0, 0.0, 0.0);
pub const GREEN: Srgba = Srgba::rgb(0.0, 1.0, 0.0);
pub const BLUE: Srgba = Srgba::rgb(0.0, 0.0, 1.0);

pub const USE_WIDE: bool = true;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, GlaciersPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (rotate, handle_input, draw))
        .run();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let res = &window.single().unwrap().resolution;
    let scale = 0.25;

    let image_size = Vec2::new(res.width() * scale, res.height() * scale).as_uvec2();
    let image = Image::new_fill(
        Extent3d {
            width: image_size.x,
            height: image_size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8; 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );

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

    let half_width = (image_size.x / 2) as f32;
    let half_height = (image_size.y / 2) as f32;

    let tri = Triangle::new([
        Vertex::new(
            Vec3::new(
                half_width - half_width / 2.0,
                half_height - half_height / 2.0,
                0.0,
            ),
            RED.into(),
        ),
        Vertex::new(
            Vec3::new(
                half_width + half_width / 2.0,
                half_height - half_height / 2.0,
                0.0,
            ),
            GREEN.into(),
        ),
        Vertex::new(
            Vec3::new(
                half_width + half_width / 2.0,
                half_height + half_height / 2.0,
                0.0,
            ),
            BLUE.into(),
        ),
    ]);
    println!("{:#?}", tri);

    // TODO rotate Transform
    commands.spawn((tri, Transform::default(), Rotates));
}

fn handle_input(keyboard: Res<ButtonInput<KeyCode>>) {
    // Exit
    if keyboard.just_pressed(KeyCode::Escape) {
        std::process::exit(1);
    }
}

fn draw(
    mut ctx: Query<&GlaciersContext>,
    mut images: ResMut<Assets<Image>>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
    triangles: Query<(&Triangle, &GlobalTransform)>,
    views: Query<(&Camera, &GlobalTransform)>,
    time: Res<Time>,
    mut timer: Local<Option<Timer>>,
) -> Result<()> {
    // let timer = *timer;
    match timer.as_mut() {
        Some(timer) => {
            timer.tick(time.delta());
        }
        None => {
            *timer = Some(Timer::from_seconds(0.25, TimerMode::Repeating));
        }
    };
    let Ok(ctx) = ctx.single_mut() else {
        return Ok(());
    };
    let mut canvas = ctx.get_canvas(&mut images);
    let Some(canvas) = canvas.as_mut() else {
        return Ok(());
    };
    let Ok((camera, global_camera)) = views.single() else {
        return Ok(());
    };

    // info!("start");
    let start = Instant::now();

    canvas.clear();

    for (triangle, transform) in &triangles {
        let new_pos_a = transform.transform_point(triangle.vertices[0].pos.to_vec3());
        let new_pos_b = transform.transform_point(triangle.vertices[1].pos.to_vec3());
        let new_pos_c = transform.transform_point(triangle.vertices[2].pos.to_vec3());
        let tri = Triangle::new([
            Vertex::new(new_pos_a, RED.into()),
            Vertex::new(new_pos_b, GREEN.into()),
            Vertex::new(new_pos_c, BLUE.into()),
        ]);
        // let view_pos = match camera.world_to_viewport_with_depth(global_camera, pos) {
        //     Ok(view_pos) => view_pos,
        //     Err(err) => {
        //         warn!("Triangle needs to be clipped. {err:?}");
        //         break;
        //     }
        // };

        if USE_WIDE {
            canvas.draw_triangle_wide(&tri);
        } else {
            canvas.draw_triangle(&tri);
        }
        for v in triangle.vertices {
            canvas.draw_point(v.pos.xy().as_uvec2(), [0xff, 0, 0xff, 1]);
        }
    }

    let frame_time = start.elapsed().as_secs_f32() * 1000.0;
    let fps = 1000.0 / frame_time;
    if let Some(timer) = timer.as_ref()
        && timer.just_finished()
    {
        window.single_mut().unwrap().title = format!(
            "Glaciers - {}x{} {:.2}ms {:.0}fps - {} triangles",
            canvas.size().x,
            canvas.size().y,
            frame_time,
            fps,
            triangles.count()
        );
    }
    // info!("end");
    Ok(())
}

#[derive(Component)]
struct Rotates;

/// Rotates any entity around the x and z axis
fn rotate(time: Res<Time>, mut query: Query<&mut Transform, With<Rotates>>) {
    let speed = 1.5;
    for mut transform in &mut query {
        // transform.rotate_x(0.55 * time.delta_secs() * speed);
        // transform.rotate_z(0.15 * time.delta_secs() * speed);
    }
}
