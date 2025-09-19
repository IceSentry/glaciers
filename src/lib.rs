use bevy::{
    asset::RenderAssetUsages,
    ecs::system::SystemParam,
    image::TextureFormatPixelInfo,
    prelude::*,
    render::{extract_component::ExtractComponent, renderer::RenderDevice},
    window::WindowResolution,
};
use wgpu::{Extent3d, TextureDimension, TextureFormat};

use crate::canvas::GlaciersCanvas;

pub mod canvas;
pub mod plugin;

#[derive(Component, Default, Clone, ExtractComponent)]
pub struct GlaciersContext {
    pub image: Handle<Image>,
    pub scale: f32,
    pub image_size: UVec2,
}

impl GlaciersContext {
    pub fn image_size_f32(&self) -> Vec2 {
        self.image_size.as_vec2()
    }
}

// TODO add some kind of submit_canvas that uploads the image to the gpu directly
#[derive(SystemParam)]
pub struct GlaciersParams<'w, 's> {
    images: ResMut<'w, Assets<Image>>,
    context: Query<'w, 's, &'static GlaciersContext>,
    _render_device: Res<'w, RenderDevice>,
}

impl<'w, 's> GlaciersParams<'w, 's> {
    pub fn init_context<'a>(
        &'a mut self,
        resolution: WindowResolution,
        scale: f32,
    ) -> GlaciersContext {
        let image_size =
            Vec2::new(resolution.width() * scale, resolution.height() * scale).as_uvec2();
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
        GlaciersContext {
            image: self.images.add(image),
            scale,
            image_size,
            ..default()
        }
    }

    pub fn context<'a>(&'a self) -> &'a GlaciersContext {
        self.context.single().unwrap()
    }

    pub fn canvas<'a>(&'a mut self) -> GlaciersCanvas<'a> {
        let context = self.context.single().unwrap();
        let image = self.images.get_mut(context.image.id()).unwrap();
        let pixel_size = image.texture_descriptor.format.pixel_size().unwrap();
        GlaciersCanvas {
            color: image,
            pixel_size,
        }
    }

    pub fn submit_canvas<'a>(&'a mut self) {
        // TODO upload image data to gpu
    }
}
