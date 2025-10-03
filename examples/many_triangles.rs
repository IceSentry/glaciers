use std::time::Instant;

use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    feathers::{
        FeathersPlugins,
        controls::checkbox,
        dark_theme::create_dark_theme,
        theme::{ThemeBackgroundColor, ThemedText, UiTheme},
    },
    prelude::*,
    ui::Checked,
    ui_widgets::{ValueChange, observe},
    window::PrimaryWindow,
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

pub const USE_WIDE: bool = true;
pub const USE_BOX: bool = true;

pub const TRIANGLE_COUNT: usize = 1000;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, GlaciersPlugin, FeathersPlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(GlobalConfigs {
            use_wide: true,
            use_box: true,
            _show_box_outline: true,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (rotate, handle_input, draw))
        .run();
}

fn setup(
    mut commands: Commands,
    mut glaciers_params: GlaciersParams,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let scale = 1.0;
    let res = window.single().unwrap().resolution.clone();
    let glaciers_context = glaciers_params.init_context(res, scale);
    let image_size = glaciers_context.image_size;

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
    fastrand::seed(42);
    let mut count = 0;
    loop {
        let random_color = Color::srgba(fastrand::f32(), fastrand::f32(), fastrand::f32(), 1.0);

        let max_size = image_size.x / 5;
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
        if tri.is_visible() {
            commands.spawn(tri);
            count += 1;
            if count == TRIANGLE_COUNT {
                break;
            }
        }
    }
    spawn_ui_root(&mut commands);
}

#[derive(Resource)]
struct GlobalConfigs {
    use_wide: bool,
    use_box: bool,
    // TODO
    _show_box_outline: bool,
}

fn spawn_ui_root(commands: &mut Commands) {
    let root = (
        ThemeBackgroundColor(bevy::feathers::tokens::WINDOW_BG),
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            padding: UiRect::all(px(8)),
            row_gap: px(8),
            // width: percent(10),
            min_width: px(100),
            ..Default::default()
        },
        children![
            (
                checkbox(Checked, Spawn((Text::new("Use wide"), ThemedText))),
                observe(
                    |change: On<ValueChange<bool>>,
                     mut commands: Commands,
                     mut configs: ResMut<GlobalConfigs>| {
                        configs.use_wide = change.value;
                        let mut checkbox = commands.entity(change.source);
                        if change.value {
                            checkbox.insert(Checked);
                        } else {
                            checkbox.remove::<Checked>();
                        }
                    }
                )
            ),
            (
                checkbox(Checked, Spawn((Text::new("Use box"), ThemedText))),
                observe(
                    |change: On<ValueChange<bool>>,
                     mut commands: Commands,
                     mut configs: ResMut<GlobalConfigs>| {
                        configs.use_box = change.value;
                        let mut checkbox = commands.entity(change.source);
                        if change.value {
                            checkbox.insert(Checked);
                        } else {
                            checkbox.remove::<Checked>();
                        }
                    }
                )
            ),
        ],
    );
    commands.spawn(root);
}

fn handle_input(keyboard: Res<ButtonInput<KeyCode>>) {
    // Exit
    if keyboard.just_pressed(KeyCode::Escape) {
        std::process::exit(1);
    }
}

fn draw(
    mut glaciers_params: GlaciersParams,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
    triangles: Query<&Triangle>,
    global_configs: Res<GlobalConfigs>,
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

    let mut canvas = glaciers_params.canvas();

    // info!("-- start --");
    let start = Instant::now();

    canvas.clear();

    {
        let _draw_triangle_span = info_span!("draw_triangle").entered();

        for triangle in &triangles {
            if global_configs.use_wide {
                if global_configs.use_box {
                    canvas.draw_triangle_wide_box(triangle, false);
                } else {
                    canvas.draw_triangle_wide(triangle);
                }
            } else {
                if global_configs.use_box {
                    canvas.draw_triangle_box(triangle, false);
                } else {
                    canvas.draw_triangle(triangle);
                }
            }
        }
    }

    let frame_time = start.elapsed().as_secs_f32() * 1000.0;
    let fps = 1000.0 / frame_time;
    if let Some(timer) = timer.as_ref()
        && timer.just_finished()
    {
        let _update_title_span = info_span!("update_window_title").entered();

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
