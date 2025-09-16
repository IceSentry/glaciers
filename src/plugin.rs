use bevy::{
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    ecs::query::QueryItem,
    prelude::*,
    render::{
        Render, RenderApp,
        extract_component::ExtractComponentPlugin,
        render_asset::RenderAssets,
        render_graph::{
            NodeRunError, RenderGraphContext, RenderGraphExt, RenderLabel, ViewNode, ViewNodeRunner,
        },
        renderer::{RenderContext, RenderDevice},
        texture::GpuImage,
        view::{ViewTarget, prepare_view_targets},
    },
    window::WindowResized,
};
use wgpu::{Extent3d, util::TextureBlitter};

use crate::GlaciersContext;

pub struct GlaciersPlugin;
impl Plugin for GlaciersPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<GlaciersContext>::default())
            .add_systems(PreUpdate, handle_resize);
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
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

fn handle_resize(
    mut ctx: Query<&GlaciersContext>,
    mut images: ResMut<Assets<Image>>,
    mut resize_events: MessageReader<WindowResized>,
) {
    // TODO handle multiple contexts
    let Ok(ctx) = ctx.single_mut() else {
        warn!("Resizing multiple context is not implemented yet");
        return;
    };
    let Some(image) = images.get_mut(ctx.image.id()) else {
        return;
    };

    for ev in resize_events.read() {
        if !(image.size_f32().x != ev.width * ctx.scale
            || image.size_f32().y != ev.height * ctx.scale)
        {
            // size hasn't actually changed
            continue;
        }

        image.resize(Extent3d {
            width: (ev.width * ctx.scale) as u32,
            height: (ev.height * ctx.scale) as u32,
            depth_or_array_layers: 1,
        });
        println!("Image size: {} ", image.size());
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct GlaciersLabel;

#[derive(Default)]
pub struct GlaciersNode;
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
pub struct GlaciersTextureBlitter(TextureBlitter);

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
