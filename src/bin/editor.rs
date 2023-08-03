#![windows_subsystem = "windows"]

use ppd_editor::editor;

fn main() {
    env_logger::init();

    let mut native_options = eframe::NativeOptions {
        centered: true,
        initial_window_size: Some(eframe::epaint::vec2(1200.0, 700.0)),
        ..Default::default()
    };

    #[cfg(not(target_os = "macos"))]
    {
        use eframe::IconData;

        native_options.icon_data = match IconData::try_from_png_bytes(include_bytes!(
            "../../build/windows/ppd-editor.ico"
        )) {
            Ok(icon) => Some(icon),
            Err(err) => {
                log::warn!("Failed to load window icon: {}", err);
                None
            }
        };
    }

    eframe::run_native(
        editor::APP_TITLE,
        native_options,
        Box::new(editor::setup_eframe),
    );
}
