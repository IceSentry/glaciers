use bevy::{
    asset::RenderAssetUsages,
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    ecs::query::QueryItem,
    image::TextureFormatPixelInfo,
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
        .add_plugins((DefaultPlugins, GlaciersPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

struct GlaciersPlugin;
impl Plugin for GlaciersPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<RasterizerImage>::default());
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
struct RasterizerImage {
    image: Handle<Image>,
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let res = &window.single().unwrap().resolution;
    let image = Image::new_fill(
        Extent3d {
            width: res.width() as u32,
            height: res.height() as u32,
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
        Transform::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(bevy::color::palettes::css::MAGENTA.into()),
            ..default()
        },
        RasterizerImage {
            image: images.add(image),
            ..default()
        },
    ));
}

fn update(
    mut rasterizer_image: Query<&RasterizerImage>,
    mut images: ResMut<Assets<Image>>,
    mut resize_events: EventReader<WindowResized>,
) -> Result<()> {
    let Some(image) = rasterizer_image
        .single_mut()
        .ok()
        .and_then(|handle| images.get_mut(handle.image.id()))
    else {
        return Ok(());
    };

    // Resize when needed
    for ev in resize_events.read() {
        image.resize(Extent3d {
            width: ev.width as u32,
            height: ev.height as u32,
            depth_or_array_layers: 1,
        });
    }

    // Clear the image
    if let Some(data) = image.data.as_mut() {
        for old_pixel in data.chunks_mut(image.texture_descriptor.format.pixel_size()) {
            old_pixel.copy_from_slice(&[0; 4]);
        }
    }

    let half_width = image.size().x / 2;
    let half_height = image.size().y / 2;

    let half_size = 32;
    for x in half_width - half_size..=half_width + half_size {
        for y in half_height - half_size..=half_height + half_size {
            image.set_color_at(x, y, Color::srgba(1.0, 0.0, 0.0, 1.0))?;
        }
    }

    Ok(())
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct GlaciersLabel;

#[derive(Default)]
struct GlaciersNode;
impl ViewNode for GlaciersNode {
    type ViewQuery = (&'static ViewTarget, &'static RasterizerImage);

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, rasterizer_image): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let texture_blitter = world.resource::<GlaciersTextureBlitter>();

        let post_process = view_target.post_process_write();

        let gpu_images = world.resource::<RenderAssets<GpuImage>>();
        let Some(image) = gpu_images.get(&rasterizer_image.image) else {
            return Ok(());
        };

        texture_blitter.copy(
            world.resource::<RenderDevice>().wgpu_device(),
            render_context.command_encoder(),
            &image.texture_view,
            // TODO blit directly to render target
            &post_process.destination,
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
