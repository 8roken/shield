use anyhow::{Result, anyhow};
use crossbeam::channel::Sender;
use std::io::BufReader;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::{ffi::CString, os::unix::net::UnixStream};

use crate::app::Event;
use pulseaudio::protocol::*;

#[allow(dead_code)]
pub struct Audio {
    socket: BufReader<UnixStream>,
    version: ProtocolVersion,
    seq: ProtocolSequence,
    sinks: Arc<RwLock<Vec<SinkInfo>>>,
}

type SinkIndex = u32;
type ProtocolVersion = u16;
type ProtocolSequence = u32;

// Audio requires 2 connections to pulseaudio: One for receiving events coming from
// the server, and one for getting information out of PulseAudio.
impl Audio {
    pub fn new() -> Result<Audio> {
        let (mut socket, version, seq) = initialize_with_client_name("shield-queries")?;
        // Finally, write a command to get the list of sinks. The reply contains the information we're after.
        write_command_message(
            socket.get_mut(),
            seq + 1,
            &Command::GetSinkInfoList,
            version,
        )?;

        let (seq, info_list) = read_reply_message::<SinkInfoList>(&mut socket, version)?;

        Ok(Audio {
            socket,
            sinks: Arc::new(RwLock::new(info_list)),
            version,
            seq,
        })
    }

    pub fn monitor(&mut self, sender: Sender<Event>) -> JoinHandle<()> {
        let sinks = self.sinks.clone();
        std::thread::spawn(|| Monitor::new(sinks, sender).unwrap().run().unwrap())
    }
}

struct Monitor {
    version: ProtocolVersion,
    sequence: ProtocolSequence,
    socket: BufReader<UnixStream>,
    sinks: Arc<RwLock<Vec<SinkInfo>>>,
    sender: Sender<Event>,
}

impl Monitor {
    fn new(sinks: Arc<RwLock<Vec<SinkInfo>>>, sender: Sender<Event>) -> Result<Self> {
        let (socket, version, seq) = initialize_with_client_name("shield-monitor")?;
        Ok(Monitor {
            socket,
            version,
            sinks,
            sender,
            sequence: seq,
        })
    }

    fn run(&mut self) -> Result<()> {
        write_command_message(
            self.socket.get_mut(),
            self.sequence + 1,
            &Command::Subscribe(SubscriptionMask::SINK),
            self.version,
        )?;

        self.sequence = read_ack_message(&mut self.socket)?;

        loop {
            let (_seq, event) = read_command_message(&mut self.socket, self.version)?;
            match event {
                Command::SubscribeEvent(event) => {
                    if let Some(index) = event.index {
                        let change = match self.switch(index) {
                            Ok(change) => change,
                            Err(err) => {
                                eprintln!("Error occured for index: #{index:?}: #{err:?}");
                                continue;
                            }
                        };

                        if change.volume_changed() {
                            self.sender.send(Event::VolumeChanged(change.volume()));
                        }
                    }
                }
                _ => eprintln!("got unexpected event {:?}", event),
            }
        }
    }

    // The SinkInfo is outdated and needs to be replaced with the current
    // state. Once done, an event should be emitted so that the UI can be
    // updated.
    fn switch(&mut self, index: SinkIndex) -> Result<Change> {
        let mut change: Option<Change> = None;

        write_command_message(
            self.socket.get_mut(),
            1,
            &Command::GetSinkInfo(GetSinkInfo {
                index: Some(index),
                name: None,
            }),
            self.version,
        )?;

        let (_seq, new_sink) = read_reply_message::<SinkInfo>(&mut self.socket, self.version)?;
        let mut sinks = self.sinks.write().unwrap();

        for sink_info in sinks.iter_mut() {
            if sink_info.index == new_sink.index {
                change = Some(Change {
                    old: sink_info.clone(),
                    new: new_sink.clone(),
                });
                *sink_info = new_sink;
                break;
            }
        }

        change.ok_or(anyhow!("Sink couldn't be found for index: {index:?}"))
    }
}

fn initialize_with_client_name(
    client_name: &str,
) -> Result<(BufReader<UnixStream>, ProtocolVersion, ProtocolSequence)> {
    let seq = 0;
    let socket_path =
        pulseaudio::socket_path_from_env().ok_or(anyhow!("PulseAudio not available"))?;
    let mut sock = std::io::BufReader::new(UnixStream::connect(socket_path)?);

    let cookie = pulseaudio::cookie_path_from_env()
        .and_then(|path| std::fs::read(path).ok())
        .unwrap_or_default();
    let auth = AuthParams {
        version: MAX_VERSION,
        supports_shm: false,
        supports_memfd: false,
        cookie,
    };

    write_command_message(sock.get_mut(), seq + 1, &Command::Auth(auth), MAX_VERSION)?;
    let (seq, auth_info) = read_reply_message::<AuthReply>(&mut sock, MAX_VERSION)?;
    let protocol_version = std::cmp::min(MAX_VERSION, auth_info.version);

    let mut props = Props::new();
    props.set(Prop::ApplicationName, CString::new(client_name).unwrap());
    write_command_message(
        sock.get_mut(),
        seq + 1,
        &Command::SetClientName(props),
        protocol_version,
    )?;

    let (seq, _) = read_reply_message::<SetClientNameReply>(&mut sock, protocol_version)?;

    Ok((sock, protocol_version, seq))
}

struct Change {
    old: SinkInfo,
    new: SinkInfo,
}

impl Change {
    pub fn volume_changed(&self) -> bool {
        self.old.cvolume.channels().first().unwrap() != self.new.cvolume.channels().first().unwrap()
    }

    pub fn volume(&self) -> f32 {
        let volume = self.new.cvolume.channels().first().unwrap().as_u32();
        volume as f32 / 0x10000 as f32
    }
}
