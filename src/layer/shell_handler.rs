use super::Layer;
use smithay_client_toolkit::shell::wlr_layer::{
    LayerShellHandler, LayerSurface, LayerSurfaceConfigure,
};

use smithay_client_toolkit::delegate_layer;
use wayland_client::{Connection, QueueHandle};

impl LayerShellHandler for Layer {
    // Unfortunately, I don't know how to trigger this with Dispatch and Wgpu as the
    // driver for the surface.
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {}

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        _configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        self.sender.send(crate::layer::gpu::Event::Configure);
    }
}

delegate_layer!(Layer);
