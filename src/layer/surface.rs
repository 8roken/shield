use super::Layer;
use crate::config::Settings;
use crate::layer::wgpu::Surface;
use anyhow::{Ok, Result, anyhow};
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{
    self, Anchor, KeyboardInteractivity, LayerShell, LayerSurface,
};
use std::ptr::NonNull;
use wayland_client::Proxy;
use wayland_client::{Connection, QueueHandle};
use wgpu::Instance;

#[derive(Default)]
pub struct Builder<'a> {
    connection: Option<&'a Connection>,
    compositor: Option<&'a CompositorState>,
    queue_handle: Option<&'a QueueHandle<Layer>>,
    layer_shell: Option<&'a LayerShell>,
    settings: Option<&'a Settings>,
    instance: Option<&'a Instance>,
}

impl<'a> Builder<'a> {
    pub fn with_connection(&'a mut self, conn: &'a Connection) -> &'a mut Self {
        self.connection = Some(conn);
        self
    }

    pub fn with_layer_shell(&'a mut self, layer_shell: &'a LayerShell) -> &'a mut Self {
        self.layer_shell = Some(layer_shell);
        self
    }

    pub fn with_compositor(&'a mut self, compositor: &'a CompositorState) -> &'a mut Self {
        self.compositor = Some(compositor);
        self
    }

    pub fn with_queue_handle(&'a mut self, qh: &'a QueueHandle<Layer>) -> &'a mut Self {
        self.queue_handle = Some(qh);
        self
    }

    pub fn with_settings(&'a mut self, settings: &'a Settings) -> &'a mut Self {
        self.settings = Some(settings);
        self
    }

    pub fn with_instance(&'a mut self, instance: &'a Instance) -> &'a mut Self {
        self.instance = Some(instance);
        self
    }

    pub fn create<'window>(&self) -> Result<(LayerSurface, Surface<'window>)> {
        let settings = self.settings.ok_or(anyhow!("Settings not present"))?;
        let layer_shell = self.layer_shell.ok_or(anyhow!("LayerShell is missing"))?;
        let queue_handle = self.queue_handle.ok_or(anyhow!("QueueHandle is missing"))?;
        let compositor = self.compositor.ok_or(anyhow!("Compositor is missing"))?;
        let connection = self
            .connection
            .ok_or(anyhow!("Wayland connection is missing"))?;
        let instance = self.instance.ok_or(anyhow!("GPU Instance is missing"))?;

        let layer_surface = layer_shell.create_layer_surface(
            queue_handle,
            compositor.create_surface(queue_handle),
            wlr_layer::Layer::Top,
            Some("trampoline:main"),
            None,
        );
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer_surface.set_anchor(Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer_surface.set_size(settings.size().0, settings.size().1);
        layer_surface.set_margin(0, settings.position().0, settings.position().1, 0);

        // Commit does so much under the hood; it binds a wl_buffer and hooks into
        // Wayland server which means the LayerSurface can be dropped by this function and
        // it will leave on.
        layer_surface.commit();

        let raw_display_handle = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
            NonNull::new(connection.backend().display_ptr() as *mut _).unwrap(),
        ));
        let raw_window_handle = RawWindowHandle::Wayland(WaylandWindowHandle::new(
            NonNull::new(layer_surface.wl_surface().id().as_ptr() as *mut _).unwrap(),
        ));

        let raw = unsafe {
            instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle,
                    raw_window_handle,
                })
                .unwrap()
        };
        Ok((layer_surface, raw))
    }
}
