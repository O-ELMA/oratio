mod app;
mod speech;
mod ui;

fn main() {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("Oratio"),
    );
    dioxus::launch(app::App);
}
