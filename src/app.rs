use std::thread::{JoinHandle, sleep};
use std::time;
use std::{sync::Arc, time::Instant};

use crate::config::Settings;
use crate::layer::{Wire, gpu};
use crossbeam::channel::{Receiver, Sender};
use derive_getters::Getters;

use super::{layer::Layer, shield::Shield};

#[derive(Debug)]
pub enum Event {
    VolumeChanged(f32),
    Hide,
}

#[derive(Getters)]
pub struct App {
    shield: Shield,
    settings: Arc<Settings>,
    receiver: Receiver<Event>,
    sender: Sender<Event>,
    handles: Vec<JoinHandle<()>>,
    transmitter: Option<Sender<gpu::Event>>,
    wire: Option<Wire>,
}

impl App {
    pub fn new(settings: Settings) -> App {
        let (sender, receiver) = crossbeam::channel::unbounded();
        let settings = Arc::new(settings);
        Self {
            receiver,
            sender,
            settings: settings.clone(),
            handles: vec![],
            transmitter: None,
            shield: Shield::new(settings),
            wire: None,
        }
    }

    pub fn width(&self) -> u32 {
        self.settings.size().0
    }

    pub fn height(&self) -> u32 {
        self.settings.size().1
    }

    pub fn register_handle(&mut self, handle: JoinHandle<()>) {
        self.handles.push(handle);
    }

    pub fn volume_changed(&mut self, volume: f32, timer_tx: Sender<Instant>) {
        let wire = match self.wire.as_mut() {
            Some(wire) => wire,
            None => {
                let layer = Layer::new(self.settings.clone()).unwrap();
                self.wire = Some(layer.run().unwrap());
                self.wire.as_mut().unwrap()
            }
        };

        let scene = self.shield.scene(volume);
        wire.sender()
            .send(crate::layer::gpu::Event::Render(scene))
            .unwrap();

        wire.sender().send(crate::layer::gpu::Event::Paint).unwrap();
        timer_tx
            .send(Instant::now() + time::Duration::from_millis(750))
            .unwrap();
    }

    pub fn start(mut self) {
        let (timer_tx, timer_rx) = crossbeam::channel::unbounded();
        let sender = self.sender.clone();
        let countdown = std::thread::spawn(move || {
            while let Ok(instant) = timer_rx.recv() {
                let duration = instant - Instant::now();
                sleep(duration);
                if timer_rx.is_empty() {
                    sender.send(Event::Hide).unwrap();
                }
            }
        });

        while let Ok(event) = self.receiver.recv() {
            match event {
                Event::VolumeChanged(volume) => self.volume_changed(volume, timer_tx.clone()),
                Event::Hide => {
                    if let Some(mut wire) = self.wire.take() {
                        wire.sender().send(crate::layer::gpu::Event::Terminate);
                    }
                }
            }
        }

        let _ = countdown.join();
    }
}
