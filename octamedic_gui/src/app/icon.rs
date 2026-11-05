use ggez::{
    GameError, GameResult,
    winit::window::{Icon, Window},
};
use image::{
    RgbaImage,
    imageops::{self, FilterType},
};

const ICON: &[u8] = include_bytes!("../../assets/icon.png");
// the title bar's, and the taskbar's on X11. Windows draws it at 16x16 times the display scaling
const WINDOW_ICON_SIZE: u32 = 64;
// Windows' taskbar and Alt+Tab, which go up to 256x256
#[cfg(windows)]
const TASKBAR_ICON_SIZE: u32 = 256;

/// Gives the window its icons on Windows and X11. macOS takes the Dock's from the app bundle and
/// Wayland from the desktop file, so there this changes nothing.
pub fn set_window_icons(window: &Window) -> GameResult {
    let image = decode()?;
    window.set_window_icon(Some(icon(&image, WINDOW_ICON_SIZE)?));
    #[cfg(windows)]
    {
        use ggez::winit::platform::windows::WindowExtWindows;
        window.set_taskbar_icon(Some(icon(&image, TASKBAR_ICON_SIZE)?));
    }
    return Ok(());
}

fn decode() -> GameResult<RgbaImage> {
    return image::load_from_memory(ICON)
        .map(|image| image.to_rgba8())
        .map_err(|error| GameError::ResourceLoadError(error.to_string()));
}

fn icon(image: &RgbaImage, size: u32) -> GameResult<Icon> {
    let scaled = imageops::resize(image, size, size, FilterType::Triangle);
    return Icon::from_rgba(scaled.into_raw(), size, size)
        .map_err(|error| GameError::ResourceLoadError(error.to_string()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_is_square() {
        let image = decode().unwrap();
        assert_eq!(image.width(), image.height());
        assert!(icon(&image, WINDOW_ICON_SIZE).is_ok());
    }
}
