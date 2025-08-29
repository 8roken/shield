use anyhow::{Result, anyhow};
use crossbeam::channel;
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::delegate_registry;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::registry_handlers;
use smithay_client_toolkit::shell::wlr_layer::{LayerShell, LayerSurface};
use std::sync::Arc;
use std::thread::JoinHandle;
use wayland_client::globals::{GlobalList, registry_queue_init};
use wayland_client::{Connection, EventQueue};
use wgpu;

mod compositor;
pub mod gpu;
use gpu::Event;

use crate::config::Settings;

mod output;
mod shell_handler;
mod surface;

#[allow(dead_code)]
pub struct Layer {
    connection: Connection,
    settings: Arc<crate::config::Settings>,
    global: GlobalList,
    layer_shell: LayerShell,
    registry_state: RegistryState,
    output_state: OutputState,
    compositor: CompositorState,
    receiver: channel::Receiver<Event>,
    sender: channel::Sender<Event>,

    layer_surface: Option<LayerSurface>,

    // Queue will be consumed when run() is called
    queue: Option<EventQueue<Self>>,
}

impl Layer {
    pub fn new(settings: Arc<Settings>) -> Result<Layer> {
        let connection = Connection::connect_to_env()?;

        let (sender, receiver) = channel::unbounded();
        let (global, queue) = registry_queue_init(&connection)?;
        let handle = queue.handle();
        let layer_shell = LayerShell::bind(&global, &handle)?;
        let compositor = CompositorState::bind(&global, &handle)?;
        let registry_state = RegistryState::new(&global);
        let output_state = OutputState::new(&global, &handle);

        let layer = Layer {
            connection: connection.clone(),
            global,
            registry_state,
            compositor,
            output_state,
            layer_shell,
            receiver,
            sender,
            settings: settings,
            queue: Some(queue),
            layer_surface: None,
        };

        Ok(layer)
    }

    pub fn run(mut self) -> Result<Wire> {
        let instance = gpu::get_instance();
        let mut queue = self.queue.take().ok_or(anyhow!("Layer already ran"))?;
        let (layer_surface, surface) = surface::Builder::default()
            .with_connection(&self.connection)
            .with_layer_shell(&self.layer_shell)
            .with_compositor(&self.compositor)
            .with_queue_handle(&queue.handle())
            .with_settings(&self.settings)
            .with_instance(&instance)
            .create()?;

        self.layer_surface = Some(layer_surface);
        let mut engine = gpu::create(&self.settings, surface, instance)?;

        let (cancel_tx, cancel_rx) = channel::bounded(1);
        let renderer_handle = {
            let mut receiver = self.receiver.clone();

            std::thread::spawn(move || {
                engine.ingest(&mut receiver).unwrap();
                cancel_tx.send(true);
            })
        };

        let sender = self.sender.clone();
        let handle = std::thread::spawn(move || {
            loop {
                if let Ok(_) = cancel_rx.try_recv() {
                    break;
                }
                // This is a busy loop for now and needs to be optimized
                // so it can use the underlying FD and sleep until it can be
                // woken up when the FD is ready to be process an event.
                // For this to work, the future will need to wait for the FD but
                // something will need to also challenge the read for the cancel_rx,
                // like crossbeam's select! so that the loop needs to be cancelled,
                // this loop doesn't need to wait for a wayland event to wake up.
                queue.flush().unwrap();
                queue.dispatch_pending(&mut self).unwrap();
                let guard = queue.prepare_read().unwrap();
                guard.read().unwrap();
                queue.dispatch_pending(&mut self).unwrap();
            }
            if let Some(layer_surface) = self.layer_surface.take() {
                drop(queue);
                drop(layer_surface);
            }
            renderer_handle.join();
        });

        Ok(Wire { handle, sender })
    }
}

#[allow(dead_code)]
pub struct Wire {
    handle: JoinHandle<()>,
    sender: channel::Sender<Event>,
}

impl Wire {
    pub fn sender(&mut self) -> &channel::Sender<Event> {
        &self.sender
    }
}

impl ProvidesRegistryState for Layer {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

delegate_registry!(Layer);
