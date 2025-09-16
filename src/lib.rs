use bevy::{
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    ecs::query::QueryItem,
    prelude::*,
    render::{
        Render, RenderApp,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::RenderAssets,
        render_graph::{
            NodeRunError, RenderGraphContext, RenderGraphExt, RenderLabel, ViewNode, ViewNodeRunner,
        },
        renderer::{RenderContext, RenderDevice},
        texture::GpuImage,
        view::{ViewTarget, prepare_view_targets},
    },
};
use wgpu::util::TextureBlitter;

#[derive(Component, Default, Clone, ExtractComponent)]
pub struct GlaciersContext {
    pub image: Handle<Image>,
    pub scale: f32,
}

pub struct GlaciersPlugin;
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
