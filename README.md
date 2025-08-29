# Shield

Wayland volume notifier similar to the badge that users of OS X/iOS are familiar with. It can be customized to fit with the overall look and feel of your Linux environment.

## Dependencies
The initial goal of this project was to learn how to build a GUI in Rust that renders to Wayland using the GPU. As such, the number of dependencies was kept to a bare minimum to learn as much as possible.

- PulseAudio/PipeWire
- Wayland Compositor

## Configurations
Shield is designed to work out of the box without configuration. Configurations is available for users that would like to tweak the look and feel.

It uses [config-rs](https://github.com/rust-cli/config-rs) to parse the configuration. As such, you write your configuration using TOML, JSON, YAML, etc.

```toml
[frame]
radius = 14

[frame.position]
y = 250

[frame.size]
width = 300
height = 200

[color]
background = [42, 40, 73, 230]
foreground = [255, 255, 255, 100]
```

