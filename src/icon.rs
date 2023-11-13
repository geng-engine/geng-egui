use super::*;

use anyhow::Result;
use egui::{ColorImage, Context, ImageData, TextureId, TextureOptions};

pub struct Icon {
    pub texture: TextureId,
    pub width: usize,
    pub height: usize,
}

impl Icon {
    pub fn from_raw(
        size: vec2<usize>,
        data: &[u8],
        alloc: &Context,
        options: geng::asset::TextureOptions,
    ) -> Result<Self> {
        let image = ColorImage::from_rgba_unmultiplied(**size, data);

        let filter = match options.filter {
            ugli::Filter::Nearest => egui::TextureFilter::Nearest,
            ugli::Filter::Linear => egui::TextureFilter::Linear,
        };
        let options = TextureOptions {
            magnification: filter,
            minification: filter,
        };

        let id = alloc.tex_manager().write().alloc(
            "Some icon".to_string(),
            ImageData::Color(Arc::new(image)),
            options,
        );

        Ok(Self {
            texture: id,
            width: size.x,
            height: size.y,
        })
    }

    pub fn from_ugli(ugli: &Ugli, texture: &ugli::Texture, alloc: &Context) -> Result<Self> {
        let size = texture.size();

        let read =
            ugli::FramebufferRead::new_color(ugli, ugli::ColorAttachmentRead::Texture(texture));
        let data = read.read_color();
        let data: Vec<u8> = (0..read.size().y)
            .flat_map(|y| {
                let data = &data;
                (0..read.size().x).flat_map(move |x| {
                    let color = data.get(x, y);
                    **color.to_vec4()
                })
            })
            .collect();

        Self::from_raw(
            size,
            &data,
            alloc,
            geng::asset::TextureOptions {
                filter: ugli::Filter::Nearest, // TODO
                wrap_mode: ugli::WrapMode::Clamp,
                premultiply_alpha: false,
            },
        )
    }

    pub fn size(&self) -> egui::Vec2 {
        egui::Vec2 {
            x: self.width as f32,
            y: self.height as f32,
        }
    }

    pub fn id(&self) -> TextureId {
        self.texture
    }
}
