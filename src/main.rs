use std::time::Instant;

use bevy::{
    asset::RenderAssetUsages,
    core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        tonemapping::Tonemapping,
    },
    ecs::query::QueryItem,
    image::TextureFormatPixelInfo,
    prelude::*,
    render::{
        Render, RenderApp,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::RenderAssets,
        render_graph::{
            NodeRunError, RenderGraphContext, RenderGraphExt, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        texture::GpuImage,
        view::{ViewTarget, prepare_view_targets},
    },
    window::{PrimaryWindow, WindowResized},
};
use wgpu::util::TextureBlitter;

pub const BLACK: Srgba = Srgba::rgb(0.0, 0.0, 0.0);
pub const WHITE: Srgba = Srgba::rgb(1.0, 1.0, 1.0);

pub const RED: Srgba = Srgba::rgb(1.0, 0.0, 0.0);
pub const GREEN: Srgba = Srgba::rgb(0.0, 1.0, 0.0);
pub const BLUE: Srgba = Srgba::rgb(0.0, 0.0, 1.0);

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            // .set(WindowPlugin {
            //     primary_window: Some(Window {
            //         present_mode: bevy::window::PresentMode::AutoNoVsync,
            //         ..Default::default()
            //     }),
            //     ..Default::default()
            // }),
            GlaciersPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_resize, handle_input, draw))
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
            .add_render_graph_node::<ViewNodeRunner<GlaciersNode>>(Core3d, GlaciersLabel)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndMainPassPostProcessing,
                    GlaciersLabel,
                    Node3d::Upscaling,
                ),
            )
            .add_systems(Render, prepare_texture_blitter.after(prepare_view_targets));
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

    let half_width = image.size_f32().x / 2.0;
    let half_height = image.size_f32().y / 2.0;

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::default(),
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
    commands.spawn(Triangle::new([
        Vertex::new(
            Vec3::new(half_width - 50.0, half_height + 40.0, 0.0),
            RED.into(),
        ),
        Vertex::new(Vec3::new(half_width, half_height - 40.0, 0.0), GREEN.into()),
        Vertex::new(
            Vec3::new(half_width + 50.0, half_height + 40.0, 0.0),
            BLUE.into(),
        ),
    ]));
}

#[derive(Clone, Copy)]
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
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut glaciers_context: Query<&GlaciersContext>,
    mut images: ResMut<Assets<Image>>,
    triangles: Query<(Entity, &Triangle)>,
) {
    if keyboard.just_pressed(KeyCode::Escape) || keyboard.just_pressed(KeyCode::KeyQ) {
        std::process::exit(1);
    }
    if keyboard.just_pressed(KeyCode::KeyR) {
        let Ok(glaciers_context) = glaciers_context.single_mut() else {
            return;
        };
        let Some(image) = images.get_mut(glaciers_context.image.id()) else {
            return;
        };

        for (e, _) in triangles {
            commands.entity(e).despawn();
        }

        let random_pos = || {
            Vec3::new(
                fastrand::f32() * image.size_f32().x,
                fastrand::f32() * image.size_f32().y,
                0.0,
            )
        };
        let _random_vertex = || Vertex {
            pos: random_pos(),
            color: LinearRgba::new(fastrand::f32(), fastrand::f32(), fastrand::f32(), 1.0),
        };
        for _ in 0..100 {
            let vertices = [
                // Vertex::new(random_pos(), RED.into()),
                // Vertex::new(random_pos(), GREEN.into()),
                // Vertex::new(random_pos(), BLUE.into()),
                _random_vertex(),
                _random_vertex(),
                _random_vertex(),
            ];
            commands.spawn(Triangle::new(vertices));
        }
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
    triangles: Query<(Entity, &Triangle)>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
) -> Result<()> {
    let Ok(ctx) = ctx.single_mut() else {
        return Ok(());
    };
    let Some(image) = images.get_mut(ctx.image.id()) else {
        return Ok(());
    };

    let start = Instant::now();

    // Clear the image
    if let Some(data) = image.data.as_mut() {
        for old_pixel in data.chunks_mut(image.texture_descriptor.format.pixel_size().unwrap()) {
            old_pixel.copy_from_slice(&[0; 4]);
        }
    }

    let half_width = image.size().x / 2;
    let half_height = image.size().y / 2;
    let _center = UVec2::new(half_width, half_height);

    for (_, triangle) in triangles {
        draw_triangle(image, triangle);
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

fn draw_triangle(
    image: &mut Image,
    Triangle {
        vertices,
        aabb: (min, max),
    }: &Triangle,
) {
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

            // This is only needed because winding order is random right now.
            // Normally you only need to check if it's > 0.0
            if (abp >= 0 && bcp >= 0 && cap >= 0) || (abp <= 0 && bcp <= 0 && cap <= 0) {
                // I actually need a Mat4x3 but glam doesn't support that
                let color = Mat4::from_cols(
                    vertices[0].color.to_vec4(),
                    vertices[1].color.to_vec4(),
                    vertices[2].color.to_vec4(),
                    Vec4::ZERO,
                ) * Vec4::new(weight_a, weight_b, weight_c, 0.0);

                draw_point(
                    image,
                    p.as_uvec2(),
                    Color::srgba(color.x, color.y, color.z, color.w),
                );
            } else {
                // draw_point(image, p.as_uvec2(), RED.into());
            }
        }
    }

    // Draw the outline useful for wireframe mode
    // draw_line(image, vertices[0].pos, vertices[1].pos, BLACK.into());
    // draw_line(image, vertices[1].pos, vertices[2].pos, BLACK.into());
    // draw_line(image, vertices[2].pos, vertices[0].pos, BLACK.into());

    // Draw each corners
    draw_point(image, vertices[0].pos.xy().as_uvec2(), RED.into());
    draw_point(image, vertices[1].pos.xy().as_uvec2(), GREEN.into());
    draw_point(image, vertices[2].pos.xy().as_uvec2(), BLUE.into());
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
    image.set_color_at(pos.x, pos.y, color).unwrap();
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct GlaciersLabel;

#[derive(Default)]
struct GlaciersNode;
impl ViewNode for GlaciersNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static GlaciersContext,
        &'static GlaciersTextureBlitter,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, glaciers_context, texture_blitter): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let gpu_images = world.resource::<RenderAssets<GpuImage>>();
        let Some(image) = gpu_images.get(&glaciers_context.image) else {
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

#[derive(Component, Deref)]
struct GlaciersTextureBlitter(TextureBlitter);

fn prepare_texture_blitter(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ViewTarget)>,
) {
    for (e, view_target) in &views {
        let texture_blitter = wgpu::util::TextureBlitter::new(
            render_device.wgpu_device(),
            view_target.main_texture_format(),
        );
        commands
            .entity(e)
            .insert(GlaciersTextureBlitter(texture_blitter));
    }
}
