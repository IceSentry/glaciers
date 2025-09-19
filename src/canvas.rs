use bevy::{image::TextureFormatPixelInfo, prelude::*};
use glam_wide::{CmpGe, Vec2x8, Vec3x8, boolf32x8, f32x8};

pub struct GlaciersCanvas<'a> {
    pub(crate) color: &'a mut Image,
    pub(crate) pixel_size: usize,
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

    pub fn draw_point(&mut self, pos: UVec2, color: [u8; 4]) {
        let width = self.color.texture_descriptor.size.width;
        let pixel_offset = pos.y * width + pos.x;
        let offset = pixel_offset as usize * self.pixel_size;

        let data = self.color.data.as_mut().unwrap();
        if offset + 3 > data.len() {
            return;
        }

        let [r, g, b, a] = color;
        data[offset + 0] = r;
        data[offset + 1] = g;
        data[offset + 2] = b;
        data[offset + 3] = a;
    }

    pub fn draw_line(&mut self, start: Vec3, end: Vec3, color: [u8; 4]) {
        let mut x0 = start.x as i32;
        let mut y0 = start.y as i32;
        let x1 = end.x as i32;
        let y1 = end.y as i32;

        if x0 == x1 {
            let sy = if y0 < y1 { 1 } else { -1 };
            while y0 < y1 {
                self.draw_point(UVec2::new(x0 as u32, y0 as u32), color);
                y0 += sy;
            }
        } else if y0 == y1 {
            let sx = if x0 < x1 { 1 } else { -1 };
            while x0 < x1 {
                self.draw_point(UVec2::new(x0 as u32, y0 as u32), color);
                x0 += sx;
            }
        } else {
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
    }

    pub fn draw_triangle_wireframe(
        &mut self,
        Triangle { vertices, .. }: &Triangle,
        color: [u8; 4],
    ) {
        self.draw_line(vertices[0].pos, vertices[1].pos, color);
        self.draw_line(vertices[1].pos, vertices[2].pos, color);
        self.draw_line(vertices[2].pos, vertices[0].pos, color);
    }

    pub fn draw_triangle(&mut self, triangle: &Triangle) {
        // returns double the signed area of the triangle
        fn edge_function(a: IVec2, b: IVec2, c: IVec2) -> i32 {
            (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
        }

        let Triangle {
            vertices,
            aabb: (min, max),
        } = triangle;
        let a = vertices[0].pos.xy().as_ivec2();
        let b = vertices[1].pos.xy().as_ivec2();
        let c = vertices[2].pos.xy().as_ivec2();
        let abc = edge_function(a, b, c);
        if abc == 0 {
            return;
        };

        // I need to use a macro because the inline annotation is not aggressive enough
        macro_rules! draw_point {
            ($x: ident, $y: ident) => {
                let p = IVec2::new($x, $y);

                let abp = edge_function(a, b, p);
                let bcp = edge_function(b, c, p);
                let cap = edge_function(c, a, p);
                if $y == 0 && $x < 4 {
                    println!("cap: {:?}", cap);
                }

                // Normally you only need to check one of these, but I don't know the winding order of
                // the triangle
                if abp >= 0 && bcp >= 0 && cap >= 0 {
                    let weights = IVec3::new(bcp, cap, abp).as_vec3a() / abc as f32;
                    let color = Mat3::from_cols(
                        vertices[0].color,
                        vertices[1].color,
                        vertices[2].color
                    ) * weights;
                    let color = [color.x, color.y, color.z, 1.0].map(|v| (v * u8::MAX as f32) as u8);

                    self.draw_point(p.as_uvec2(), color);
                }
            };
        }

        println!("--- start ---");

        // This should probably be relative to resolution scale
        let block_size: i32 = 8;
        let orient = (max.x - min.x) / (max.y - min.y);
        if orient >= 0.4 && orient <= 1.6 {
            for y in (min.y as i32..=max.y as i32).step_by(block_size as usize) {
                let mut pass = false;
                for x in (min.x as i32..=max.x as i32).step_by(block_size as usize) {
                    let c00 = IVec2::new(x, y);
                    let c01 = IVec2::new(x, y + block_size - 1);
                    let c10 = IVec2::new(x + block_size - 1, y);
                    let c11 = IVec2::new(x + block_size - 1, y + block_size - 1);

                    let _draw_corners = |canvas: &mut Self, color| {
                        canvas.draw_line(c00.extend(0).as_vec3(), c01.extend(0).as_vec3(), color);
                        canvas.draw_line(c01.extend(0).as_vec3(), c11.extend(0).as_vec3(), color);
                        canvas.draw_line(c11.extend(0).as_vec3(), c10.extend(0).as_vec3(), color);
                        canvas.draw_line(c10.extend(0).as_vec3(), c00.extend(0).as_vec3(), color);
                    };

                    let corners = [c00, c01, c10, c11].map(|p| {
                        let abp = edge_function(a, b, p);
                        let bcp = edge_function(b, c, p);
                        let cap = edge_function(c, a, p);
                        abp >= 0 && bcp >= 0 && cap >= 0
                    });
                    if corners.iter().any(|&c| c) {
                        // at least one point is inside the triangle
                        for y in c00.y as i32..=c11.y as i32 {
                            for x in c00.x as i32..=c11.x as i32 {
                                draw_point!(x, y);
                            }
                        }
                        // draw_corners(self, [0, 0xff, 0, 0xff]);
                        pass = true;
                    } else {
                        // draw_corners(self, [0xff, 0, 0, 0xff]);
                        if pass {
                            break;
                        }
                    }
                }
            }
        } else {
            for y in min.y as i32..=max.y as i32 {
                for x in min.x as i32..=max.x as i32 {
                    draw_point!(x, y);
                }
            }
        }
    }

    pub fn draw_triangle_wide(&mut self, triangle: &Triangle) {
        // returns double the signed area of the triangle
        fn edge_function(a: Vec2, b: Vec2, c: Vec2) -> f32 {
            (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
        }

        fn edge_function_wide(a: Vec2x8, b: Vec2x8, c: Vec2x8) -> f32x8 {
            (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
        }

        let Triangle {
            vertices,
            aabb: (min, max),
        } = triangle;
        let a = vertices[0].pos.xy();
        let b = vertices[1].pos.xy();
        let c = vertices[2].pos.xy();

        let abc = edge_function(a, b, c);
        if abc == 0.0 {
            return;
        };

        let a = Vec2x8::new_splat(a.x as f32, a.y as f32);
        let b = Vec2x8::new_splat(b.x as f32, b.y as f32);
        let c = Vec2x8::new_splat(c.x as f32, c.y as f32);
        let abc = edge_function_wide(a, b, c);

        let color_a = Vec3x8::splat(vertices[0].color);
        let color_b = Vec3x8::splat(vertices[1].color);
        let color_c = Vec3x8::splat(vertices[2].color);

        const SIMD_SIZE: usize = 8;
        for y in min.y as i32..=max.y as i32 {
            for x in (min.x as i32..=max.x as i32).step_by(SIMD_SIZE) {
                let mut x_wide = [0.0_f32; SIMD_SIZE];
                for i in 0..SIMD_SIZE {
                    x_wide[i] = x as f32 + i as f32;
                }
                let p_wide = Vec2x8::new(f32x8::new(x_wide), f32x8::splat(y as f32));

                let abp = edge_function_wide(a, b, p_wide);
                let bcp = edge_function_wide(b, c, p_wide);
                let cap = edge_function_wide(c, a, p_wide);

                let weights = Vec3x8::new(bcp, cap, abp) / abc;
                let r = color_a.x * weights.x + color_b.x * weights.y + color_c.x * weights.z;
                let g = color_a.y * weights.x + color_b.y * weights.y + color_c.y * weights.z;
                let b = color_a.z * weights.x + color_b.z * weights.y + color_c.z * weights.z;

                let abp_ge = boolf32x8::from(abp.cmp_ge(0.0));
                let bcp_ge = boolf32x8::from(bcp.cmp_ge(0.0));
                let cap_ge = boolf32x8::from(cap.cmp_ge(0.0));
                let check = abp_ge & bcp_ge & cap_ge;

                if !check.any() {
                    // All lanes are false which means there's nothing to draw
                    continue;
                }

                // Unwiden stuff and draw the points
                let color: [Vec3; SIMD_SIZE] = Vec3x8::new(r, g, b).into();
                let ps: [Vec2; SIMD_SIZE] = p_wide.into();
                let check = check.to_array();

                for i in 0..SIMD_SIZE {
                    if check[i] {
                        self.draw_point(
                            ps[i].as_uvec2(),
                            [color[i].x, color[i].y, color[i].z, 1.0]
                                .map(|v| (v * u8::MAX as f32) as u8),
                        );
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub pos: Vec3,
    pub color: Vec3,
}

impl Vertex {
    pub fn new(pos: Vec3, color: Color) -> Self {
        Self {
            pos: pos,
            color: color.to_linear().to_vec3(),
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
