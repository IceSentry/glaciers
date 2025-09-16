use bevy::{image::TextureFormatPixelInfo, prelude::*};

pub struct GlaciersCanvas<'a> {
    pub(crate) color: &'a mut Image,
    // depth: Image,
}

impl<'a> GlaciersCanvas<'a> {
    pub fn size(&self) -> UVec2 {
        self.color.size()
    }

    pub fn size_f32(&self) -> Vec2 {
        self.color.size_f32()
    }

    pub fn clear(&mut self) {
        if let Some(data) = self.color.data.as_mut() {
            for old_pixel in
                data.chunks_mut(self.color.texture_descriptor.format.pixel_size().unwrap())
            {
                old_pixel.copy_from_slice(&[0; 4]);
            }
        }
    }

    pub fn draw_point(&mut self, pos: UVec2, color: Color) {
        let _ = self.color.set_color_at(pos.x, pos.y, color);
    }

    pub fn draw_line(&mut self, start: Vec3, end: Vec3, color: Color) {
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
            self.draw_point(UVec2::new(x0 as u32, y0 as u32), color);

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

    pub fn draw_triangle_wireframe(&mut self, Triangle { vertices, .. }: &Triangle, color: Color) {
        self.draw_line(vertices[0].pos, vertices[1].pos, color);
        self.draw_line(vertices[1].pos, vertices[2].pos, color);
        self.draw_line(vertices[2].pos, vertices[0].pos, color);
    }

    // TODO benchmark and optimize
    pub fn draw_triangle(&mut self, triangle: &Triangle) {
        // returns double the signed area of the triangle
        fn edge_function(a: IVec2, b: IVec2, c: IVec2) -> i32 {
            (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
        }

        let Triangle {
            vertices,
            aabb: (min, max),
        } = triangle;

        // Only check the pixels inside the AABB
        for x in min.x as i32..=max.x as i32 {
            for y in min.y as i32..=max.y as i32 {
                let a = vertices[0].pos.xy().as_ivec2();
                let b = vertices[1].pos.xy().as_ivec2();
                let c = vertices[2].pos.xy().as_ivec2();

                let p = IVec2::new(x, y);

                let abp = edge_function(a, b, p);
                let bcp = edge_function(b, c, p);
                let cap = edge_function(c, a, p);

                let abc = edge_function(a, b, c);

                // Normally you only need to check one of these, but I don't know the winding order of
                // the triangle
                if (abp >= 0 && bcp >= 0 && cap >= 0) || (abp <= 0 && bcp <= 0 && cap <= 0) {
                    let weight_a = bcp as f32 / abc as f32;
                    let weight_b = cap as f32 / abc as f32;
                    let weight_c = abp as f32 / abc as f32;

                    let weights = Vec3::new(weight_a, weight_b, weight_c);
                    let color = Mat3::from_cols(
                        vertices[0].color.to_vec3(),
                        vertices[1].color.to_vec3(),
                        vertices[2].color.to_vec3(),
                    ) * weights;
                    let alpha = Vec3::new(
                        vertices[0].color.alpha(),
                        vertices[1].color.alpha(),
                        vertices[2].color.alpha(),
                    )
                    .dot(weights);

                    self.draw_point(p.as_uvec2(), Color::srgba(color.x, color.y, color.z, alpha));
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
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

#[derive(Component, Debug)]
pub struct Triangle {
    pub vertices: [Vertex; 3],
    pub aabb: (Vec3, Vec3),
}

impl Triangle {
    pub fn new(vertices: [Vertex; 3]) -> Self {
        let aabb = vertices.iter().fold(
            (vertices[0].pos, vertices[0].pos),
            |(prev_min, prev_max), point| (point.pos.min(prev_min), point.pos.max(prev_max)),
        );
        Self { vertices, aabb }
    }
}
