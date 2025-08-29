use crate::config::Settings;
use anyhow::{Result, anyhow};
use crossbeam::channel;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use vello::{Renderer, peniko};
use wgpu::util::TextureBlitter;
use wgpu::{self, Adapter, Backends, Device, Instance, Queue, Surface, Texture, TextureUsages};

pub enum Event {
    Configure,
    Paint,
    Render(vello::Scene),
    Terminate,
}

pub struct Engine<'a> {
    device: Device,
    adapter: Adapter,
    queue: Queue,
    surface: Surface<'a>,
    ready: Arc<AtomicBool>,
    size: (u32, u32),
    renderer: Renderer,
    texture: Texture,
}

pub fn get_instance() -> Instance {
    Instance::new(&wgpu::InstanceDescriptor {
        backends: Backends::all(),
        ..Default::default()
    })
}

pub fn create<'a>(
    settings: &Arc<Settings>,
    surface: Surface<'a>,
    instance: Instance,
) -> Result<Engine<'a>> {
    // Pick a supported adapter
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: Some(&surface),
        ..Default::default()
    }))
    .ok_or(anyhow!("Adapter could not be requested"))?;

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            ..Default::default()
        },
        None,
    ))?;

    let renderer = Renderer::new(&device, vello::RendererOptions::default())?;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: settings.size().0,
            height: settings.size().1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        label: Some("Back Buffer"),
        view_formats: &[],
    });

    Ok(Engine {
        device,
        adapter,
        queue,
        renderer,
        surface,
        texture,
        ready: Arc::new(false.into()),
        size: *settings.size(),
    })
}

impl Engine<'_> {
    pub fn paint(&self, staged_view: wgpu::TextureView) {
        let texture = self
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");

        let surface_texture = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Reencoding format"),
            });
        let blitter = TextureBlitter::new(&self.device, wgpu::TextureFormat::Rgba8UnormSrgb);

        blitter.copy(&self.device, &mut encoder, &staged_view, &surface_texture);

        self.queue.submit(Some(encoder.finish()));

        texture.present();
    }

    pub fn ingest(&mut self, receiver: &mut channel::Receiver<Event>) -> Result<()> {
        use std::sync::atomic::Ordering;

        loop {
            match receiver.recv()? {
                Event::Paint => {
                    if self.ready.load(Ordering::Relaxed) {
                        self.paint(
                            self.texture
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                        );
                    }
                }

                Event::Render(scene) => {
                    self.renderer
                        .render_to_texture(
                            &self.device,
                            &self.queue,
                            &scene,
                            &self
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                            &vello::RenderParams {
                                base_color: peniko::color::palette::css::TRANSPARENT,
                                width: self.size.0,
                                height: self.size.1,
                                antialiasing_method: vello::AaConfig::Msaa16,
                            },
                        )
                        .expect("Failed to render to a texture");
                }
                Event::Terminate => {
                    break;
                }
                Event::Configure => {
                    let width = NonZeroU32::new(self.size.0).map_or(256, NonZeroU32::get);
                    let height = NonZeroU32::new(self.size.1).map_or(256, NonZeroU32::get);

                    let adapter = &self.adapter;
                    let surface = &self.surface;
                    let device = &self.device;

                    let cap = surface.get_capabilities(adapter);
                    let surface_config = wgpu::SurfaceConfiguration {
                        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_DST,
                        format: cap.formats[0],
                        view_formats: vec![cap.formats[0]],
                        alpha_mode: wgpu::CompositeAlphaMode::PreMultiplied,
                        width: width,
                        height: height,
                        desired_maximum_frame_latency: 2,
                        present_mode: wgpu::PresentMode::Mailbox,
                    };

                    surface.configure(device, &surface_config);
                    self.ready.store(true, Ordering::Relaxed);
                    self.paint(
                        self.texture
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                    );
                }
            }
        }

        Ok(())
    }
}

impl Drop for Engine<'_> {
    fn drop(&mut self) {
        self.texture.destroy();
    }
}
