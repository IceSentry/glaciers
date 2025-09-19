use std::time::Instant;

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
    let scale = 1.0;

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
    fastrand::seed(42);
    let seed = fastrand::u64(..);
    for i in 0..1000 {
        fastrand::seed(i + seed);

        let random_color = Color::srgba(fastrand::f32(), fastrand::f32(), fastrand::f32(), 1.0);
        // let random_color = Color::WHITE;

        let max_size = image_size.x / 6;
        let random_translation = Vec3::new(
            fastrand::u32(0..image_size.x - max_size) as f32,
            fastrand::u32(0..image_size.y - max_size) as f32,
            1.0,
        );
        let pos_a = Vec3::new(
            fastrand::u32(0..max_size) as f32,
            fastrand::u32(0..max_size) as f32,
            1.0,
        ) + random_translation;
        let pos_b = Vec3::new(
            fastrand::u32(0..max_size) as f32,
            fastrand::u32(0..max_size) as f32,
            1.0,
        ) + random_translation;
        let pos_c = Vec3::new(
            fastrand::u32(0..max_size) as f32,
            fastrand::u32(0..max_size) as f32,
            1.0,
        ) + random_translation;

        let tri = Triangle::new([
            Vertex::new(pos_a, random_color),
            Vertex::new(pos_b, random_color),
            Vertex::new(pos_c, random_color),
        ]);
        commands.spawn(tri);
    }
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
    triangles: Query<&Triangle>,
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

    // info!("-- start --");
    let start = Instant::now();

    canvas.clear();

    for triangle in &triangles {
        if USE_WIDE {
            canvas.draw_triangle_wide(triangle);
        } else {
            canvas.draw_triangle(triangle);
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
    // info!("-- end --");
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
