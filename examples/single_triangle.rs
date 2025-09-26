use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    feathers::{
        FeathersPlugins,
        controls::{CheckboxProps, SliderProps, checkbox, slider},
        dark_theme::create_dark_theme,
        theme::{ThemeBackgroundColor, ThemedText, UiTheme},
    },
    prelude::*,
    ui::Checked,
    ui_widgets::{Callback, SliderPrecision, SliderStep, SliderValue, ValueChange},
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

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins, GlaciersPlugin))
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(GlobalConfigs {
            use_wide: true,
            use_box: true,
            _show_box_outline: true,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (exit_on_esc, draw))
        .run();
}

fn setup(
    mut commands: Commands,
    mut glaciers_params: GlaciersParams,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let scale = 0.5;
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

    let half_width = image_size.x / 2;
    let half_height = image_size.y / 2;
    let tri = Triangle::new([
        Vertex::new(
            Vec3::new(
                (half_width) as f32,
                (half_height - half_height / 2) as f32,
                0.0,
            ),
            RED.into(),
        ),
        Vertex::new(
            Vec3::new(
                (half_width - half_width / 2) as f32,
                (half_height + half_height / 2) as f32,
                0.0,
            ),
            GREEN.into(),
        ),
        Vertex::new(
            Vec3::new(
                (half_width + half_width / 2) as f32,
                (half_height + half_height / 2) as f32,
                0.0,
            ),
            BLUE.into(),
        ),
    ]);
    commands.spawn(tri);
    spawn_ui_root(
        &mut commands,
        image_size.x as f32,
        image_size.y as f32,
        &tri,
    );
}

#[derive(Resource)]
struct GlobalConfigs {
    use_wide: bool,
    use_box: bool,
    // TODO
    _show_box_outline: bool,
}

fn spawn_ui_root(commands: &mut Commands, max_width: f32, max_height: f32, triangle: &Triangle) {
    let root = (
        ThemeBackgroundColor(bevy::feathers::tokens::WINDOW_BG),
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            padding: UiRect::all(px(8)),
            row_gap: px(8),
            width: percent(25),
            min_width: px(200),
            ..Default::default()
        },
        children![
            checkbox(
                CheckboxProps {
                    on_change: Callback::System(commands.register_system(
                        |change: In<ValueChange<bool>>,
                         mut commands: Commands,
                         mut configs: ResMut<GlobalConfigs>| {
                            configs.use_wide = change.value;
                            if change.value {
                                commands.entity(change.source).insert(Checked);
                            } else {
                                commands.entity(change.source).remove::<Checked>();
                            }
                        }
                    )),
                },
                Checked,
                Spawn((Text::new("Use wide"), ThemedText))
            ),
            checkbox(
                CheckboxProps {
                    on_change: Callback::System(commands.register_system(
                        |change: In<ValueChange<bool>>,
                         mut commands: Commands,
                         mut configs: ResMut<GlobalConfigs>| {
                            configs.use_box = change.value;
                            if change.value {
                                commands.entity(change.source).insert(Checked);
                            } else {
                                commands.entity(change.source).remove::<Checked>();
                            }
                        }
                    )),
                },
                Checked,
                Spawn((Text::new("Use box"), ThemedText))
            ),
            // TODO add divider
            point_slider(commands, 0, &max_width, &max_height, &triangle.vertices),
            point_slider(commands, 1, &max_width, &max_height, &triangle.vertices),
            point_slider(commands, 2, &max_width, &max_height, &triangle.vertices)
        ],
    );
    commands.spawn(root);
}

fn point_slider(
    commands: &mut Commands,
    p: usize,
    max_width: &f32,
    max_height: &f32,
    triangle_vertices: &[Vertex; 3],
) -> impl Bundle {
    let handle_x_change = commands.register_system(
        move |change: In<ValueChange<f32>>,
              mut commands: Commands,
              mut triangle: Single<&mut Triangle>| {
            commands
                .entity(change.source)
                .insert(SliderValue(change.value));
            // TODO store vertex_id in slider entity and only have one handler
            triangle.vertices[p].pos.x = change.value;
            triangle.recompute_aabb();
        },
    );
    let handle_y_change = commands.register_system(
        move |change: In<ValueChange<f32>>,
              mut commands: Commands,
              mut triangle: Single<&mut Triangle>| {
            commands
                .entity(change.source)
                .insert(SliderValue(change.value));
            triangle.vertices[p].pos.y = change.value;
            triangle.recompute_aabb();
        },
    );

    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            ..Default::default()
        },
        children![
            Text(format!("Point {p}:")),
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    justify_content: JustifyContent::Start,
                    ..Default::default()
                },
                children![
                    labelled_slider(
                        "X: ",
                        *max_width,
                        triangle_vertices[p].pos.x,
                        handle_x_change
                    ),
                    labelled_slider(
                        "Y: ",
                        *max_height,
                        triangle_vertices[p].pos.y,
                        handle_y_change
                    ),
                ],
            ),
        ],
    )
}

fn labelled_slider(
    label: &str,
    max: f32,
    value: f32,
    on_change: bevy::ecs::system::SystemId<In<ValueChange<f32>>>,
) -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            width: Val::Percent(100.0),
            ..Default::default()
        },
        children![
            Text(label.into()),
            slider(
                SliderProps {
                    max,
                    value,
                    on_change: Callback::System(on_change),
                    ..Default::default()
                },
                (SliderStep(1.), SliderPrecision(1)),
            ),
        ],
    )
}

fn exit_on_esc(keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.just_pressed(KeyCode::Escape) {
        std::process::exit(1);
    }
}

fn draw(
    mut glaciers_params: GlaciersParams,
    triangle: Single<&Triangle>,
    configs: Res<GlobalConfigs>,
) -> Result<()> {
    let mut canvas = glaciers_params.canvas();
    canvas.clear();

    if configs.use_wide {
        if configs.use_box {
            canvas.draw_triangle_wide_box(&triangle);
        } else {
            canvas.draw_triangle_wide(&triangle);
        }
    } else {
        canvas.draw_triangle(&triangle);
    }

    Ok(())
}
