use egui::{Context, Visuals};
use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::State;
use wgpu::{Device, Queue, TextureFormat};
use winit::{event::WindowEvent, window::Window};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Sphere,
    Torus,
    Tetrahedron,
    Cube,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Texture {
    Glass,
    Wood,
    Steel,
    Ice,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub gravity: f32,
    pub shape: Shape,
    pub texture: Texture,
    pub light_color: [u8; 3],
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            gravity: 1.0,
            shape: Shape::Sphere,
            texture: Texture::Glass,
            light_color: [255, 255, 255],
        }
    }
}

pub struct Gui {
    pub ctx: Context,
    state: State,
    renderer: Renderer,
}

impl Gui {
    pub fn new(
        window: Arc<Window>,
        device: &Device,
        format: TextureFormat,
    ) -> Self {
        let ctx = Context::default();
        
        // Use a nice visual style
        ctx.set_visuals(Visuals::light());

        let viewport_id = ctx.viewport_id();
        let state = State::new(ctx.clone(), viewport_id, &window, Some(window.scale_factor() as f32), None, None);
        
        let renderer = Renderer::new(device, format, None, 1, false);

        Self {
            ctx,
            state,
            renderer,
        }
    }

    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        let response = self.state.on_window_event(window, event);
        response.consumed
    }

    pub fn render(
        &mut self,
        device: &Device,
        queue: &Queue,
        window: &Window,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        run_ui: impl FnMut(&Context),
    ) {
        let raw_input = self.state.take_egui_input(window);
        
        let full_output = self.ctx.run(raw_input, run_ui);

        self.state.handle_platform_output(window, full_output.platform_output);

        let clipped_primitives = self.ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [window.inner_size().width, window.inner_size().height],
            pixels_per_point: window.scale_factor() as f32,
        };

        for (id, delta) in &full_output.textures_delta.set {
            self.renderer.update_texture(device, queue, *id, delta);
        }

        self.renderer.update_buffers(
            device,
            queue,
            encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // SAFETY: Workaround for lifetime mismatch with egui-wgpu
            let pass_static: &mut wgpu::RenderPass<'static> = unsafe { std::mem::transmute(&mut pass) };
            self.renderer.render(pass_static, &clipped_primitives, &screen_descriptor);
        }
        
        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }
    }
}
