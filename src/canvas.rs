use bevy::{color::palettes::css::MAGENTA, image::TextureFormatPixelInfo, prelude::*};
use glam_wide::{CmpLe, Vec2x8, Vec3x8, boolf32x8, f32x8};

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
        let _canvas_clear_span = info_span!("canvas_clear").entered();
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

        if !triangle.is_visible() {
            return;
        }

        let Triangle {
            vertices,
            aabb: (min, max),
        } = triangle;
        let a = vertices[0].pos.xy().as_ivec2();
        let b = vertices[1].pos.xy().as_ivec2();
        let c = vertices[2].pos.xy().as_ivec2();
        let abc = edge_function(a, b, c);

        for y in min.y as i32..=max.y as i32 {
            for x in min.x as i32..=max.x as i32 {
                let p = IVec2::new(x, y);

                let abp = edge_function(a, b, p);
                let bcp = edge_function(b, c, p);
                let cap = edge_function(c, a, p);

                if abp <= 0 && bcp <= 0 && cap <= 0 {
                    let weights = IVec3::new(bcp, cap, abp).as_vec3a() / abc as f32;
                    let color =
                        Mat3::from_cols(vertices[0].color, vertices[1].color, vertices[2].color)
                            * weights;
                    let color =
                        [color.x, color.y, color.z, 1.0].map(|v| (v * u8::MAX as f32) as u8);

                    self.draw_point(p.as_uvec2(), color);
                }
            }
        }
    }

    pub fn draw_triangle_box(&mut self, triangle: &Triangle, show_outline: bool) {
        // returns double the signed area of the triangle
        fn edge_function(a: IVec2, b: IVec2, c: IVec2) -> i32 {
            (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
        }

        if !triangle.is_visible() {
            return;
        }

        let Triangle {
            vertices,
            aabb: (min, max),
        } = triangle;
        let a = vertices[0].pos.xy().as_ivec2();
        let b = vertices[1].pos.xy().as_ivec2();
        let c = vertices[2].pos.xy().as_ivec2();
        let abc = edge_function(a, b, c);

        // I need to use a macro because the inline annotation is not aggressive enough
        macro_rules! draw_point {
            ($x: ident, $y: ident) => {
                let p = IVec2::new($x, $y);

                let abp = edge_function(a, b, p);
                let bcp = edge_function(b, c, p);
                let cap = edge_function(c, a, p);

                if abp <= 0 && bcp <= 0 && cap <= 0 {
                    let weights = IVec3::new(bcp, cap, abp).as_vec3a() / abc as f32;
                    let color =
                        Mat3::from_cols(vertices[0].color, vertices[1].color, vertices[2].color)
                            * weights;
                    let color =
                        [color.x, color.y, color.z, 1.0].map(|v| (v * u8::MAX as f32) as u8);

                    self.draw_point(p.as_uvec2(), color);
                }
            };
        }

        // println!("--- start ---");

        // TODO use the same algorithm as the wide_box
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

                    let draw_corners = |canvas: &mut Self, color| {
                        canvas.draw_line(c00.extend(0).as_vec3(), c01.extend(0).as_vec3(), color);
                        canvas.draw_line(c01.extend(0).as_vec3(), c11.extend(0).as_vec3(), color);
                        canvas.draw_line(c11.extend(0).as_vec3(), c10.extend(0).as_vec3(), color);
                        canvas.draw_line(c10.extend(0).as_vec3(), c00.extend(0).as_vec3(), color);
                    };

                    let corners = [c00, c01, c10, c11].map(|p| {
                        let abp = edge_function(a, b, p);
                        let bcp = edge_function(b, c, p);
                        let cap = edge_function(c, a, p);
                        abp <= 0 && bcp <= 0 && cap <= 0
                    });
                    if corners.iter().any(|&c| c) {
                        // at least one point is inside the triangle
                        for y in c00.y as i32..=c11.y as i32 {
                            for x in c00.x as i32..=c11.x as i32 {
                                draw_point!(x, y);
                            }
                        }
                        if show_outline {
                            draw_corners(self, [0, 0xff, 0, 0xff]);
                        }
                        pass = true;
                    } else {
                        if show_outline {
                            draw_corners(self, [0xff, 0, 0, 0xff]);
                        }
                        if pass {
                            break;
                        }
                    }
                }
            }
        } else {
            if show_outline {
                self.draw_line(
                    Vec3::new(min.x, min.y, 0.0),
                    Vec3::new(min.x, max.y, 0.0),
                    [0, 0xff, 0, 0xff],
                );
                self.draw_line(
                    Vec3::new(min.x, max.y, 0.0),
                    Vec3::new(max.x, max.y, 0.0),
                    [0, 0xff, 0, 0xff],
                );
                self.draw_line(
                    Vec3::new(max.x, max.y, 0.0),
                    Vec3::new(max.x, min.y, 0.0),
                    [0, 0xff, 0, 0xff],
                );
                self.draw_line(
                    Vec3::new(max.x, min.y, 0.0),
                    Vec3::new(min.x, min.y, 0.0),
                    [0, 0xff, 0, 0xff],
                );
            }

            for y in min.y as i32..=max.y as i32 {
                for x in min.x as i32..=max.x as i32 {
                    draw_point!(x, y);
                }
            }
        }
    }

    pub fn draw_triangle_wide(&mut self, triangle: &Triangle) {
        fn edge_function_wide(a: Vec2x8, b: Vec2x8, c: Vec2x8) -> f32x8 {
            (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
        }

        if !triangle.is_visible() {
            return;
        }

        let Triangle {
            vertices,
            aabb: (min, max),
        } = triangle;
        let a = vertices[0].pos.xy();
        let b = vertices[1].pos.xy();
        let c = vertices[2].pos.xy();

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

                // Assumes winding order is CCW
                // TODO need to make winding order configurable
                let abp_cmp = boolf32x8::from(abp.cmp_le(0.0));
                let bcp_cmp = boolf32x8::from(bcp.cmp_le(0.0));
                let cap_cmp = boolf32x8::from(cap.cmp_le(0.0));
                let check = abp_cmp & bcp_cmp & cap_cmp;

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

    pub fn draw_triangle_wide_box(&mut self, triangle: &Triangle, show_outline: bool) {
        const SIMD_SIZE: usize = 8;
        const BLOCK_SIZE: i32 = SIMD_SIZE as i32;

        fn edge_function_wide(a: Vec2x8, b: Vec2x8, c: Vec2x8) -> f32x8 {
            (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
        }

        if !triangle.is_visible() {
            return;
        };

        let Triangle {
            vertices,
            aabb: (min, max),
        } = triangle;
        let a = vertices[0].pos.xy();
        let b = vertices[1].pos.xy();
        let c = vertices[2].pos.xy();

        let a = Vec2x8::new_splat(a.x as f32, a.y as f32);
        let b = Vec2x8::new_splat(b.x as f32, b.y as f32);
        let c = Vec2x8::new_splat(c.x as f32, c.y as f32);
        let abc = edge_function_wide(a, b, c);

        let color_a = Vec3x8::splat(vertices[0].color);
        let color_b = Vec3x8::splat(vertices[1].color);
        let color_c = Vec3x8::splat(vertices[2].color);

        let draw_block = |canvas: &mut Self, x, y| {
            let mut has_drawn = false;
            let c00 = IVec2::new(x, y);
            let c01 = IVec2::new(x, y + BLOCK_SIZE - 1);
            let c10 = IVec2::new(x + BLOCK_SIZE - 1, y);
            let c11 = IVec2::new(x + BLOCK_SIZE - 1, y + BLOCK_SIZE - 1);

            let draw_corners = |canvas: &mut Self, color| {
                canvas.draw_line(c00.extend(0).as_vec3(), c01.extend(0).as_vec3(), color);
                canvas.draw_line(c01.extend(0).as_vec3(), c11.extend(0).as_vec3(), color);
                canvas.draw_line(c11.extend(0).as_vec3(), c10.extend(0).as_vec3(), color);
                canvas.draw_line(c10.extend(0).as_vec3(), c00.extend(0).as_vec3(), color);
            };

            for y in c00.y as i32..=c11.y as i32 {
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

                // Assumes winding order is CCW
                // TODO need to make winding order configurable
                let abp_cmp = boolf32x8::from(abp.cmp_le(0.0));
                let bcp_cmp = boolf32x8::from(bcp.cmp_le(0.0));
                let cap_cmp = boolf32x8::from(cap.cmp_le(0.0));
                let check = abp_cmp & bcp_cmp & cap_cmp;

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
                        has_drawn = true;
                        canvas.draw_point(
                            ps[i].as_uvec2(),
                            [color[i].x, color[i].y, color[i].z, 1.0]
                                .map(|v| (v * u8::MAX as f32) as u8),
                        );
                    }
                }
            }
            if show_outline {
                if has_drawn {
                    draw_corners(canvas, [0, 0xff, 0, 0xff]);
                } else {
                    draw_corners(canvas, [0xff, 0, 0, 0xff]);
                }
            }
            has_drawn
        };

        let mut top_start_x = min.x;
        let mut bottom_start_x = min.x;
        for v in triangle.vertices {
            if v.pos.y == min.y {
                top_start_x = v.pos.x;
            }
            if v.pos.y == max.y {
                bottom_start_x = v.pos.x;
            }
        }

        let mut y = min.y as i32;
        let mut x = top_start_x as i32;
        let half_y = (min.y as i32 + ((max.y as i32 - min.y as i32) >> 1));
        loop {
            let mut min_x = min.x as i32;
            let mut max_x = max.x as i32;
            // WARN this isn't inclusive so I have to handroll it :(
            // for x in (min_x..x).rev().step_by(BLOCK_SIZE as usize) {
            let mut x_loop = x - BLOCK_SIZE;
            loop {
                if !draw_block(self, x_loop, y) {
                    min_x = x_loop + BLOCK_SIZE;
                    break;
                }
                if x_loop < min.x as i32 {
                    break;
                }
                x_loop -= BLOCK_SIZE;
            }
            for x in (x..=max.x as i32 + BLOCK_SIZE).step_by(BLOCK_SIZE as usize) {
                if !draw_block(self, x, y) {
                    max_x = x;
                    break;
                }
            }

            // let min_start = Vec3::new(min_x as f32, y as f32, 0.0);
            // let max_start = Vec3::new(max_x as f32, y as f32, 0.0);
            // let end_offset = Vec3::new(0.0, BLOCK_SIZE as f32, 0.0);
            // self.draw_line(min_start, min_start + end_offset, [0xff, 0xff, 0, 0]);
            // self.draw_line(max_start, max_start + end_offset, [0xff, 0, 0xff, 0]);

            y += BLOCK_SIZE;
            if y >= half_y {
                break;
            }

            x = min_x + ((max_x - min_x) >> 1);
        }
        let mut x = bottom_start_x as i32;
        let mut y = max.y as i32;
        loop {
            let mut min_x = min.x as i32;
            let mut max_x = max.x as i32;
            // WARN this isn't inclusive so I have to handroll it :(
            // for x in (min_x..x).rev().step_by(BLOCK_SIZE as usize) {
            let mut x_loop = x - BLOCK_SIZE;
            loop {
                if !draw_block(self, x_loop, y) {
                    min_x = x_loop + BLOCK_SIZE;
                    break;
                }
                if x_loop < min.x as i32 {
                    break;
                }
                x_loop -= BLOCK_SIZE;
            }
            for x in (x..=max.x as i32 + BLOCK_SIZE).step_by(BLOCK_SIZE as usize) {
                if !draw_block(self, x, y) {
                    max_x = x;
                    break;
                }
            }

            // let min_start = Vec3::new(min_x as f32, y as f32, 0.0);
            // let max_start = Vec3::new(max_x as f32, y as f32, 0.0);
            // let end_offset = Vec3::new(0.0, BLOCK_SIZE as f32, 0.0);
            // self.draw_line(min_start, min_start + end_offset, [0xff, 0xff, 0, 0]);
            // self.draw_line(max_start, max_start + end_offset, [0xff, 0, 0xff, 0]);

            y -= BLOCK_SIZE;
            if y < half_y {
                break;
            }
            x = min_x + ((max_x - min_x) >> 1);
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

#[derive(Component, Debug, Clone, Copy)]
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

    pub fn recompute_aabb(&mut self) {
        let aabb = self.vertices.iter().fold(
            (self.vertices[0].pos, self.vertices[0].pos),
            |(prev_min, prev_max), point| (point.pos.min(prev_min), point.pos.max(prev_max)),
        );
        self.aabb = aabb;
    }

    pub fn is_visible(&self) -> bool {
        fn edge_function(a: Vec2, b: Vec2, c: Vec2) -> f32 {
            (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
        }
        let a = self.vertices[0].pos.xy();
        let b = self.vertices[1].pos.xy();
        let c = self.vertices[2].pos.xy();

        let abc = edge_function(a, b, c);
        abc < 0.0
    }
}
