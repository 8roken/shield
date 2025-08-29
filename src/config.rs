use anyhow::Result;
use config::*;
use derive_getters::Getters;
use std::cmp;
use vello::peniko::Color;
use vello::peniko::color::{AlphaColor, Srgb};

#[derive(Getters, Debug)]
pub struct Settings {
    size: (u32, u32),
    position: (i32, i32),
    radius: f64,
    background_color: AlphaColor<Srgb>,
    foreground_color: AlphaColor<Srgb>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            size: (300, 300),
            position: (0, 250),
            radius: 14.0,
            background_color: Color::from_rgba8(42, 40, 68, 220),
            foreground_color: Color::from_rgba8(255, 255, 255, 150),
        }
    }
}

impl Settings {
    pub fn new(path: Option<String>) -> Result<Settings> {
        let mut settings = Self::default();

        let mut builder = Config::builder()
            .add_source(File::with_name("~/.config/shield/config").required(false));

        if let Some(path) = path {
            builder = builder.add_source(File::with_name(&path));
        }

        let config = builder.build()?;
        set_size(&mut settings.size, &config);
        set_position(&mut settings.position, &config);
        set_radius(&mut settings.radius, &config);
        set_color(&mut settings.background_color, &config, "color.background");
        set_color(&mut settings.foreground_color, &config, "color.foreground");

        Ok(settings)
    }
}
fn set_radius(radius: &mut f64, config: &Config) {
    if let Ok(mut value) = config.get_float("frame.radius") {
        value = value.max(0.0);
        value = value.min(40.0);

        *radius = value;
    }
}

fn set_size(size: &mut (u32, u32), config: &Config) {
    if let Ok(mut height) = config.get_int("frame.size.height") {
        height = cmp::min(height, 400);
        height = cmp::max(height, 40);
        size.1 = height as u32;
    }

    if let Ok(mut width) = config.get_int("frame.size.width") {
        width = cmp::min(width, 800);
        width = cmp::max(width, 100);
        size.0 = width as u32;
    }
}

// y cannot be negative as the shield
// is positionned at the bottom of the screen by default.
fn set_position(position: &mut (i32, i32), config: &Config) {
    if let Ok(x) = config.get_int("frame.position.x") {
        position.0 = x as i32;
    }

    if let Ok(mut y) = config.get_int("frame.position.y") {
        y = cmp::max(y, 0);
        position.1 = y as i32;
    }
}

fn set_color(color: &mut AlphaColor<Srgb>, config: &Config, key: &str) {
    if let Ok(mut value) = config.get_array(key) {
        let components: Vec<u8> = value
            .into_iter()
            .map(|v| v.into_uint().unwrap_or(0) as u8)
            .collect();

        *color = match components.len() {
            3 => Color::from_rgb8(components[0], components[1], components[2]),
            4 => Color::from_rgba8(components[0], components[1], components[2], components[3]),
            _ => {
                eprintln!(
                    "Invalid color supplied, it should be a RGB(A) format: {:?}",
                    components
                );
                return;
            }
        }
    }
}
