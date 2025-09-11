use std::time::Instant;

use bevy::{
    asset::RenderAssetUsages,
    color::palettes::css::{BLACK, BLUE, GREEN, RED, WHITE},
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    ecs::query::QueryItem,
    image::TextureFormatPixelInfo,
    math::bounding::Aabb2d,
    prelude::*,
    render::{
        RenderApp,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::RenderAssets,
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        texture::GpuImage,
        view::ViewTarget,
    },
    window::{PrimaryWindow, WindowResized},
};
use wgpu::util::TextureBlitter;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: bevy::window::PresentMode::AutoNoVsync,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            GlaciersPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

struct GlaciersPlugin;
impl Plugin for GlaciersPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<GlaciersContext>::default());
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        // TODO move render graph to separate module
        // TODO consider using a custom graph on the camera
        render_app
            .init_resource::<GlaciersTextureBlitter>()
            .add_render_graph_node::<ViewNodeRunner<GlaciersNode>>(Core3d, GlaciersLabel)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::Tonemapping,
                    GlaciersLabel,
                    Node3d::EndMainPassPostProcessing,
                ),
            );
    }
}

#[derive(Component, Default, Clone, ExtractComponent)]
struct GlaciersContext {
    image: Handle<Image>,
    scale: f32,
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let res = &window.single().unwrap().resolution;
    let scale = 0.25;
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
    println!("Image size: {}", image.size());

    let point_a = UVec2::new(
        fastrand::u32(0..image.size().x),
        fastrand::u32(0..image.size().y),
    );

    let point_b = UVec2::new(
        fastrand::u32(0..image.size().x),
        fastrand::u32(0..image.size().y),
    );

    let point_c = UVec2::new(
        fastrand::u32(0..image.size().x),
        fastrand::u32(0..image.size().y),
    );

    commands.spawn(Triangle {
        points: [point_a, point_b, point_c],
    });

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        GlaciersContext {
            image: images.add(image),
            scale,
            ..default()
        },
    ));
}

#[derive(Component)]
struct Triangle {
    points: [UVec2; 3],
}

fn update(
    mut commands: Commands,
    mut glaciers_context: Query<&GlaciersContext>,
    mut images: ResMut<Assets<Image>>,
    mut resize_events: EventReader<WindowResized>,
    triangles: Query<(Entity, &Triangle)>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
) -> Result<()> {
    let Ok(glaciers_context) = glaciers_context.single_mut() else {
        return Ok(());
    };
    let Some(image) = images.get_mut(glaciers_context.image.id()) else {
        return Ok(());
    };

    if keyboard.just_pressed(KeyCode::KeyR) {
        for (e, _) in triangles {
            commands.entity(e).despawn();
        }
        commands.spawn(Triangle {
            points: [
                UVec2::new(
                    fastrand::u32(0..image.size().x),
                    fastrand::u32(0..image.size().y),
                ),
                UVec2::new(
                    fastrand::u32(0..image.size().x),
                    fastrand::u32(0..image.size().y),
                ),
                UVec2::new(
                    fastrand::u32(0..image.size().x),
                    fastrand::u32(0..image.size().y),
                ),
            ],
        });
    }

    // Resize when needed
    for ev in resize_events.read() {
        if image.size_f32().x != ev.width * glaciers_context.scale
            || image.size_f32().y != ev.height * glaciers_context.scale
        {
            image.resize(Extent3d {
                width: (ev.width * glaciers_context.scale) as u32,
                height: (ev.height * glaciers_context.scale) as u32,
                depth_or_array_layers: 1,
            });
            println!("Image size: {} ", image.size());
        }
    }

    let start = Instant::now();

    // Clear the image
    if let Some(data) = image.data.as_mut() {
        for old_pixel in data.chunks_mut(image.texture_descriptor.format.pixel_size()) {
            old_pixel.copy_from_slice(&[0; 4]);
        }
    }

    let half_width = image.size().x / 2;
    let half_height = image.size().y / 2;
    let _center = UVec2::new(half_width, half_height);

    for (_, triangle) in triangles {
        draw_triangle(image, triangle.points);
    }

    window.single_mut().unwrap().title = format!(
        "Glaciers - {}x{} {:.3}ms",
        image.size().x,
        image.size().y,
        start.elapsed().as_secs_f32() / 1000.0
    );

    Ok(())
}

fn edge_function(a: IVec2, b: IVec2, c: IVec2) -> i32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn draw_triangle(image: &mut Image, points: [UVec2; 3]) {
    // Compute AABB of the triangle
    let (min, max) = points
        .iter()
        .fold((points[0], points[0]), |(prev_min, prev_max), point| {
            (point.min(prev_min), point.max(prev_max))
        });

    // Only check the pixels inside the AABB
    for x in min.x..=max.x {
        for y in min.y..=max.y {
            let a = points[0].as_ivec2();
            let b = points[1].as_ivec2();
            let c = points[2].as_ivec2();

            let p = UVec2::new(x, y).as_ivec2();
            let abp = edge_function(a, b, p);
            let bcp = edge_function(b, c, p);
            let cap = edge_function(c, a, p);

            // This is only needed because winding order is random right now.
            // Normally you only need to check if it's > 0.0
            if (abp >= 0 && bcp >= 0 && cap >= 0) || (abp <= 0 && bcp <= 0 && cap <= 0) {
                draw_point(image, p.as_uvec2(), WHITE.into());
            } else {
                draw_point(image, p.as_uvec2(), RED.into());
            }
        }
    }

    // Draw the outline useful for wireframe mode
    draw_line(image, points[0], points[1], BLACK.into());
    draw_line(image, points[1], points[2], BLACK.into());
    draw_line(image, points[2], points[0], BLACK.into());

    // Draw each corners
    draw_point(image, points[0], RED.into());
    draw_point(image, points[1], GREEN.into());
    draw_point(image, points[2], BLUE.into());
}

fn draw_line(image: &mut Image, start: UVec2, end: UVec2, color: Color) {
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
    image.set_color_at(pos.x, pos.y, color).unwrap();
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct GlaciersLabel;

#[derive(Default)]
struct GlaciersNode;
impl ViewNode for GlaciersNode {
    type ViewQuery = (&'static ViewTarget, &'static GlaciersContext);

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, rasterizer_image): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let texture_blitter = world.resource::<GlaciersTextureBlitter>();

        let gpu_images = world.resource::<RenderAssets<GpuImage>>();
        let Some(image) = gpu_images.get(&rasterizer_image.image) else {
            return Ok(());
        };

        texture_blitter.copy(
            world.resource::<RenderDevice>().wgpu_device(),
            render_context.command_encoder(),
            &image.texture_view,
            &view_target.main_texture_view(),
        );

        Ok(())
    }
}

#[derive(Resource, Deref)]
struct GlaciersTextureBlitter(TextureBlitter);

impl FromWorld for GlaciersTextureBlitter {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let texture_blitter = wgpu::util::TextureBlitter::new(
            &render_device.wgpu_device(),
            TextureFormat::bevy_default(),
        );
        Self(texture_blitter)
    }
}
