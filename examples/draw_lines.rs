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
        .add_systems(Update, (handle_input, draw))
        .run();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let res = &window.single().unwrap().resolution;
    let scale = 0.15;

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

    // info!("start");
    let start = Instant::now();

    let image_size = canvas.size();
    let half_width = (image_size.x / 2) as f32;
    let half_height = (image_size.y / 2) as f32;

    canvas.clear();

    canvas.draw_line(
        Vec3::new(0.0, half_height, 0.0),
        Vec3::new(image_size.x as f32, half_height, 0.0),
        [0xff, 0, 0, 1],
    );
    canvas.draw_line(
        Vec3::new(half_width, 0.0, 0.0),
        Vec3::new(half_width, image_size.y as f32, 0.0),
        [0, 0xff, 0, 1],
    );
    canvas.draw_line(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(image_size.x as f32, image_size.y as f32, 0.0),
        [0, 0, 0xff, 1],
    );

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
