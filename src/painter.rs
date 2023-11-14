use super::*;

/// A callback function that can be used to compose an [`egui::PaintCallback`] for custom rendering.
///
/// The callback is passed, the [`egui::PaintCallbackInfo`] and the [`GraphicsContext`] which can be used to
/// access the OpenGL context.
pub struct CallbackFn {
    #[allow(clippy::type_complexity)]
    f: Box<dyn Fn(egui::PaintCallbackInfo, &mut ugli::Framebuffer) + Sync + Send>,
}

impl CallbackFn {
    pub fn new(
        callback: impl Fn(egui::PaintCallbackInfo, &mut ugli::Framebuffer) + Sync + Send + 'static,
    ) -> Self {
        let f = Box::new(callback);
        CallbackFn { f }
    }
}

pub struct Painter {
    geng: Geng,
    textured_program: ugli::Program,
    // egui_texture_version: u64,
    textures: HashMap<egui::TextureId, ugli::Texture>,
}

impl Painter {
    pub fn new(geng: &Geng) -> Self {
        Self {
            geng: geng.clone(),
            textured_program: geng
                .shader_lib()
                .compile(include_str!("textured.glsl"))
                .unwrap(),
            // egui_texture_version: 0,
            textures: HashMap::new(),
        }
    }

    pub fn paint_and_update_textures(
        &mut self,
        framebuffer: &mut ugli::Framebuffer,
        primtives: Vec<egui::ClippedPrimitive>,
        textures_delta: &egui::TexturesDelta,
        context: &egui::Context,
    ) {
        for (id, image_delta) in &textures_delta.set {
            self.set_texture(*id, image_delta);
        }

        self.paint(framebuffer, primtives, context);

        for &id in &textures_delta.free {
            self.free_texture(id);
        }
    }

    pub fn paint(
        &mut self,
        framebuffer: &mut ugli::Framebuffer,
        primitives: Vec<egui::ClippedPrimitive>,
        context: &egui::Context,
    ) {
        let screen_size_in_pixels = framebuffer.size().map(|x| x as f32);
        // let screen_size_in_points = (
        //     screen_size_in_pixels.x / context.pixels_per_point(),
        //     screen_size_in_pixels.y / context.pixels_per_point(),
        // );

        // Render mesh
        for clipped in primitives {
            match clipped.primitive {
                egui::epaint::Primitive::Mesh(mesh) => {
                    self.paint_job(framebuffer, clipped.clip_rect, mesh)
                }
                egui::epaint::Primitive::Callback(callback) => {
                    let info = egui::PaintCallbackInfo {
                        viewport: callback.rect,
                        clip_rect: clipped.clip_rect,
                        pixels_per_point: context.pixels_per_point(),
                        screen_size_px: [
                            screen_size_in_pixels.0.round() as _,
                            screen_size_in_pixels.1.round() as _,
                        ],
                    };

                    if let Some(callback) = callback.callback.downcast_ref::<CallbackFn>() {
                        (callback.f)(info, framebuffer);
                    } else {
                        log::warn!("Unsupported render callback. Expected geng_egui::CallbackFn");
                    }
                }
            }
        }
    }

    fn paint_job(
        &mut self,
        framebuffer: &mut ugli::Framebuffer,
        clip_rect: egui::Rect,
        mesh: egui::epaint::Mesh,
    ) {
        let framebuffer_size = framebuffer.size().map(|x| x as f32);

        // Convert egui clip_rect to geng clip_aabb
        let clip_aabb = Aabb2::from_corners(
            pos_to_vec(clip_rect.min, framebuffer_size.y),
            pos_to_vec(clip_rect.max, framebuffer_size.y),
        )
        .map(|x| x as usize);

        // Get font texture
        let texture = match mesh.texture_id {
            egui::TextureId::Managed(id) => match self.textures.get(&mesh.texture_id) {
                Some(texture) => texture,
                None => {
                    log::error!("egui texture {id:?} not found");
                    return;
                }
            },
            egui::TextureId::User(_id) => todo!(),
        };

        // Convert egui vertices to geng vertices
        let vertex_shift = clip_aabb.bottom_left().map(|x| x as f32);
        let vertices: Vec<_> = mesh
            .indices
            .into_iter()
            .map(|i| {
                let mut vertex = textured_vertex(mesh.vertices[i as usize], framebuffer_size.y);
                vertex.a_pos -= vertex_shift; // Because mask is applied relative to the origin
                vertex
            })
            .collect();

        // Render triangles
        ugli::draw(
            framebuffer,
            &self.textured_program,
            ugli::DrawMode::Triangles,
            &ugli::VertexBuffer::new_dynamic(self.geng.ugli(), vertices),
            (
                ugli::uniforms! {
                    u_color: Rgba::WHITE,
                    u_texture: texture,
                    u_framebuffer_size: clip_aabb.size(),
                    u_model_matrix: mat3::identity(),
                },
                geng::PixelPerfectCamera.uniforms(clip_aabb.size().map(|x| x as f32)),
            ),
            ugli::DrawParameters {
                blend_mode: Some(ugli::BlendMode::straight_alpha()),
                viewport: Some(clip_aabb),
                ..default()
            },
        );
    }

    pub fn set_texture(&mut self, tex_id: egui::TextureId, delta: &egui::epaint::ImageDelta) {
        let [_w, _h] = delta.image.size();
        let filter = match delta.options.magnification {
            egui::TextureFilter::Nearest => ugli::Filter::Nearest,
            egui::TextureFilter::Linear => ugli::Filter::Linear,
        };

        if let Some([_x, _y]) = delta.pos {
            // Partial update
            if let Some(texture) = self.textures.get_mut(&tex_id) {
                texture.set_filter(filter);

                match &delta.image {
                    egui::ImageData::Color(image) => {
                        assert_eq!(
                            image.width() * image.height(),
                            image.pixels.len(),
                            "Mismatch between texture size and texel count"
                        );
                        todo!();
                        // let data: &[u8] = bytemuck::cast_slice(image.pixels.as_ref());
                        // texture.update_texture_part(ctx, x as _, y as _, w as _, h as _, data);
                    }
                    egui::ImageData::Font(image) => {
                        assert_eq!(
                            image.width() * image.height(),
                            image.pixels.len(),
                            "Mismatch between texture size and texel count"
                        );

                        todo!();
                        // let data: Vec<u8> = image
                        //     .srgba_pixels(None)
                        //     .flat_map(|a| a.to_array())
                        //     .collect();

                        // texture.update_texture_part(ctx, x as _, y as _, w as _, h as _, &data);
                    }
                }
            } else {
                log::error!("Failed to find egui texture {tex_id:?}");
            }
        } else {
            // New texture (or full update).
            match &delta.image {
                egui::ImageData::Color(image) => {
                    assert_eq!(
                        image.width() * image.height(),
                        image.pixels.len(),
                        "Mismatch between texture size and texel count"
                    );

                    let mut texture = ugli::Texture::new_with(
                        self.geng.ugli(),
                        vec2(image.width(), image.height()),
                        |pixel| {
                            let color = image.pixels
                                [pixel.x + (image.height() - 1 - pixel.y) * image.width()]; // Geng textures have origin in the bottom-left
                            Rgba::new(color.r(), color.g(), color.b(), color.a()).convert()
                        },
                    );
                    texture.set_filter(filter);
                    self.textures.insert(tex_id, texture);
                }
                egui::ImageData::Font(image) => {
                    assert_eq!(
                        image.width() * image.height(),
                        image.pixels.len(),
                        "Mismatch between texture size and texel count"
                    );

                    let mut texture = ugli::Texture::new_with(
                        self.geng.ugli(),
                        vec2(image.width(), image.height()),
                        |pixel| {
                            let alpha = image.pixels
                                [pixel.x + (image.height() - 1 - pixel.y) * image.width()]; // Geng textures have origin in the bottom-left
                            Rgba::new(1.0, 1.0, 1.0, alpha)
                        },
                    );
                    texture.set_filter(filter);
                    self.textures.insert(tex_id, texture);
                }
            }
        }
    }

    pub fn free_texture(&mut self, tex_id: egui::TextureId) {
        if let Some(_old_tex) = self.textures.remove(&tex_id) {}
    }
}

fn textured_vertex(egui_vertex: egui::epaint::Vertex, height: f32) -> draw2d::TexturedVertex {
    draw2d::TexturedVertex {
        a_pos: pos_to_vec(egui_vertex.pos, height),
        a_vt: pos_to_vec(egui_vertex.uv, 1.0),
        a_color: Rgba::new(
            egui_vertex.color.r(),
            egui_vertex.color.g(),
            egui_vertex.color.b(),
            egui_vertex.color.a(),
        )
        .convert(),
    }
}
