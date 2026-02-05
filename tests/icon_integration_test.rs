//! Integration test for icon support

use cosmic_desktop_widget::icons::{Icon, IconCache};

#[test]
fn test_icon_loading_and_caching() {
    let cache = IconCache::new();

    // Load weather icons
    let clear = cache.get_or_create("weather-clear", 24);
    assert!(clear.is_ok(), "Failed to load weather-clear icon");

    let clouds = cache.get_or_create("weather-clouds", 32);
    assert!(clouds.is_ok(), "Failed to load weather-clouds icon");

    // Load battery icons
    let battery = cache.get_or_create("battery-full", 24);
    assert!(battery.is_ok(), "Failed to load battery-full icon");

    // Load media icons
    let play = cache.get_or_create("media-play", 24);
    assert!(play.is_ok(), "Failed to load media-play icon");

    // Cache hit test
    let clear2 = cache.get_or_create("weather-clear", 24);
    assert!(clear2.is_ok());

    // Icon not found
    let invalid = cache.get_or_create("nonexistent", 24);
    assert!(invalid.is_err());
}

#[test]
fn test_icon_svg_rendering() {
    // Simple SVG test
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="10" fill="red"/>
    </svg>"#;

    let icon = Icon::from_svg(svg, 24);
    assert!(icon.is_ok(), "Failed to render simple SVG");

    let icon = icon.unwrap();
    assert_eq!(icon.pixmap().width(), 24);
    assert_eq!(icon.pixmap().height(), 24);
}

#[test]
fn test_icon_resize() {
    let cache = IconCache::new();
    let icon = cache.get_or_create("weather-clear", 24).unwrap();

    // Get original size
    assert_eq!(icon.pixmap().width(), 24);

    // Resize to different size
    let resized = icon.resize(48);
    assert!(resized.is_ok(), "Failed to resize icon");

    let resized = resized.unwrap();
    assert_eq!(resized.pixmap().width(), 48);
    assert_eq!(resized.pixmap().height(), 48);
}
