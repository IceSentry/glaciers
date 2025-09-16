use bevy::{prelude::*, render::extract_component::ExtractComponent};

use crate::canvas::GlaciersCanvas;

pub mod canvas;
pub mod plugin;

#[derive(Component, Default, Clone, ExtractComponent)]
pub struct GlaciersContext {
    pub image: Handle<Image>,
    pub scale: f32,
}

impl GlaciersContext {
    pub fn get_canvas<'a>(&self, images: &'a mut Assets<Image>) -> Option<GlaciersCanvas<'a>> {
        let image = images.get_mut(self.image.id())?;
        Some(GlaciersCanvas {
            color: image,
            // depth: todo!(),
        })
    }
}
