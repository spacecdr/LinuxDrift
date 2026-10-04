//! Small cached overlay: rasterize only on minute/size changes; one GPU quad per frame.
use crate::config::ClockSettings;
use chrono::{Local, Timelike};
use fontdue::{Font, FontSettings};
use std::time::Duration;

pub struct ClockOverlay {
    settings: ClockSettings,
    font: Font,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    rect: wgpu::Buffer,
    binding: Option<wgpu::BindGroup>,
    cache: Option<(u32, u32, u32, u32)>,
    extent: (u32, u32),
    anchor: (f32, f32),
    next_move: Duration,
}

impl ClockOverlay {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        settings: ClockSettings,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("LinuxDrift clock"),
            source: wgpu::ShaderSource::Wgsl(include_str!("clock.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("LinuxDrift clock"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let layout = pipeline.get_bind_group_layout(0);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let rect = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Clock position"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            settings,
            font: Font::from_bytes(
                include_bytes!("../assets/Inter-Light.ttf") as &[u8],
                FontSettings::default(),
            )
            .expect("Bundled Inter font"),
            pipeline,
            layout,
            sampler,
            rect,
            binding: None,
            cache: None,
            extent: (1, 1),
            anchor: (0.5, 0.5),
            next_move: Duration::ZERO,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        width: u32,
        height: u32,
        elapsed: Duration,
    ) {
        let now = Local::now();
        let key = (now.hour(), now.minute(), width, height);
        if self.cache != Some(key) {
            let (w, h, pixels) = rasterize(
                &self.font,
                &format!("{:02}:{:02}", key.0, key.1),
                width,
                height,
                &self.settings,
            );
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Clock cached text and backdrop"),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(w * 4),
                    rows_per_image: Some(h),
                },
                texture.size(),
            );
            self.binding = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Clock overlay"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &texture.create_view(&Default::default()),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.rect.as_entire_binding(),
                    },
                ],
            }));
            self.extent = (w, h);
            self.cache = Some(key);
            log::info!(
                "Clock texture refreshed: {:02}:{:02} ({}x{})",
                key.0,
                key.1,
                w,
                h
            );
        }
        if self.settings.position == "random" {
            if elapsed >= self.next_move {
                // Exclude the previous neighbourhood, avoiding consecutive near-identical positions.
                let previous = self.anchor;
                for _ in 0..32 {
                    self.anchor = (rand::random::<f32>(), rand::random::<f32>());
                    if (self.anchor.0 - previous.0).abs() + (self.anchor.1 - previous.1).abs() > 0.4
                    {
                        break;
                    }
                }
                self.next_move = elapsed + Duration::from_secs(self.settings.move_interval);
                log::info!("Clock moved: {:.3}, {:.3}", self.anchor.0, self.anchor.1);
            }
        } else {
            self.anchor = fixed_anchor(&self.settings.position);
        }
        let [x, y, w, h] = placement(width, height, self.extent, self.anchor);
        let rect = [
            x / width as f32 * 2.0 - 1.0,
            1.0 - y / height as f32 * 2.0,
            w / width as f32 * 2.0,
            -h / height as f32 * 2.0,
        ];
        queue.write_buffer(&self.rect, 0, bytemuck::cast_slice(&rect));
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Clock overlay"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, self.binding.as_ref().unwrap(), &[]);
        pass.draw(0..6, 0..1);
    }
}

fn fixed_anchor(position: &str) -> (f32, f32) {
    match position {
        "top" => (0.5, 0.0),
        "bottom" => (0.5, 1.0),
        "top-left" => (0.0, 0.0),
        "top-right" => (1.0, 0.0),
        "bottom-left" => (0.0, 1.0),
        "bottom-right" => (1.0, 1.0),
        _ => (0.5, 0.5),
    }
}
fn placement(width: u32, height: u32, extent: (u32, u32), anchor: (f32, f32)) -> [f32; 4] {
    let margin = (width.min(height) as f32 * 0.025).min(40.0);
    let scale = ((width as f32 - 2.0 * margin) / extent.0 as f32)
        .min((height as f32 - 2.0 * margin) / extent.1 as f32)
        .min(1.0);
    let w = extent.0 as f32 * scale;
    let h = extent.1 as f32 * scale;
    [
        margin + (width as f32 - 2.0 * margin - w) * anchor.0,
        margin + (height as f32 - 2.0 * margin - h) * anchor.1,
        w,
        h,
    ]
}
fn backdrop_alpha(x: f32, y: f32, settings: &ClockSettings) -> f32 {
    // A rounded rectangle, with a controllable fade to fully transparent edges.
    let qx = x.abs() - 0.78;
    let qy = y.abs() - 0.78;
    let distance = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - 0.22;
    let feather = (settings.background_feather / 100.0).max(0.001);
    let t = (-distance / feather).clamp(0.0, 1.0);
    settings.background_opacity * t * t * (3.0 - 2.0 * t)
}
fn rasterize(
    font: &Font,
    text: &str,
    width: u32,
    height: u32,
    settings: &ClockSettings,
) -> (u32, u32, Vec<u8>) {
    let px = (width.min(height) as f32 * settings.size / 100.0).clamp(8.0, 512.0);
    let padding = (px * 0.65).ceil() as u32;
    // Use equal digit advances, so changing minutes never shifts the clock's box.
    let digit_width = (0..=9)
        .map(|n| font.metrics(char::from(b'0' + n), px).advance_width)
        .fold(0.0_f32, f32::max);
    let colon_width = font.metrics(':', px).advance_width;
    let w = (digit_width * 4.0 + colon_width).ceil() as u32 + padding * 2;
    let h = px.ceil() as u32 + padding * 2;
    let mut image = vec![0_u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let alpha = backdrop_alpha(
                (x as f32 + 0.5) / w as f32 * 2.0 - 1.0,
                (y as f32 + 0.5) / h as f32 * 2.0 - 1.0,
                settings,
            );
            image[((y * w + x) * 4 + 3) as usize] = (alpha * 255.0).round() as u8;
        }
    }
    let mut pen = padding as f32;
    let baseline = padding as i32 + (px * 0.86).round() as i32;
    for ch in text.chars() {
        let (m, bitmap) = font.rasterize(ch, px);
        let advance = if ch == ':' { colon_width } else { digit_width };
        let left = (pen + (advance - m.advance_width) * 0.5).round() as i32 + m.xmin;
        let top = baseline - m.ymin - m.height as i32;
        for gy in 0..m.height {
            for gx in 0..m.width {
                let (x, y) = (left + gx as i32, top + gy as i32);
                if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
                    continue;
                }
                let coverage = bitmap[gy * m.width + gx];
                let i = ((y as u32 * w + x as u32) * 4) as usize;
                image[i] = coverage;
                image[i + 1] = coverage;
                image[i + 2] = coverage;
                image[i + 3] = (coverage as f32
                    + image[i + 3] as f32 * (1.0 - coverage as f32 / 255.0))
                    .round() as u8;
            }
        }
        pen += advance;
    }
    (w, h, image)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stays_on_screen_at_every_size_and_position() {
        for (w, h) in [
            (1, 1),
            (80, 60),
            (640, 480),
            (1920, 1080),
            (3840, 2160),
            (300, 1200),
        ] {
            for anchor in [(0., 0.), (1., 1.), (0.5, 0.5), (0.2, 0.8)] {
                let [x, y, cw, ch] = placement(w, h, (900, 300), anchor);
                assert!(
                    x >= 0. && y >= 0. && x + cw <= w as f32 + 0.01 && y + ch <= h as f32 + 0.01
                );
            }
        }
    }
    #[test]
    fn backdrop_fades_and_text_is_premultiplied() {
        let settings = ClockSettings {
            background_opacity: 0.7,
            ..Default::default()
        };
        assert_eq!(backdrop_alpha(1., 0., &settings), 0.);
        assert!((backdrop_alpha(0., 0., &settings) - 0.7).abs() < 0.001);
        let font = Font::from_bytes(
            include_bytes!("../assets/Inter-Light.ttf") as &[u8],
            FontSettings::default(),
        )
        .unwrap();
        let (w, h, data) = rasterize(&font, "23:59", 1920, 1080, &settings);
        assert_eq!(data.len(), (w * h * 4) as usize);
        assert!(data.chunks_exact(4).any(|p| p[0] > 240));
        assert!(data.chunks_exact(4).all(|p| p[0] <= p[3]));
        let (w2, h2, _) = rasterize(&font, "11:11", 1920, 1080, &settings);
        assert_eq!((w, h), (w2, h2));
    }
}
