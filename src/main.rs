use clap::Parser;

mod app;
pub use app::App;

mod audio;
mod config;
mod layer;
mod shield;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config: Option<String>,
}

fn main() {
    let args = Args::parse();

    let mut settings = config::Settings::new(args.config).unwrap();

    let mut app = App::new(settings);

    let mut audio = audio::Audio::new().unwrap();
    app.register_handle(audio.monitor(app.sender().clone()));

    app.start()
}
