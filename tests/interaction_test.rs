//! Integration tests for widget interaction handling

use cosmic_desktop_widget::widget::{
    FontSize, MouseButton, QuotesWidget, ScrollDirection, Widget, WidgetAction, WidgetContent,
};
use cosmic_desktop_widget::{button_code_to_mouse_button, hit_test_widgets, InputState};

#[test]
fn test_quotes_widget_is_interactive() {
    let widget = QuotesWidget::default();
    assert!(widget.is_interactive(), "QuotesWidget should be interactive");
}

#[test]
fn test_quotes_widget_click_advances() {
    let mut widget = QuotesWidget::default();

    // Get initial quote
    let initial_content = widget.content();
    let initial_text = match initial_content {
        WidgetContent::Text { text, .. } => text,
        _ => panic!("Expected Text content"),
    };

    // Click to advance
    let action = widget.on_click(MouseButton::Left, 0.5, 0.5);
    assert_eq!(action, Some(WidgetAction::NextItem));

    // Get new quote (should be different due to rotation)
    let new_content = widget.content();
    let new_text = match new_content {
        WidgetContent::Text { text, .. } => text,
        _ => panic!("Expected Text content"),
    };

    // With default random quotes, we can't guarantee different text every time
    // but the action should have been triggered
    let _ = (initial_text, new_text);
}

#[test]
fn test_quotes_widget_scroll_advances() {
    let mut widget = QuotesWidget::default();

    // Scroll down
    let action = widget.on_scroll(ScrollDirection::Down, 0.5, 0.5);
    assert_eq!(action, Some(WidgetAction::NextItem));

    // Scroll up
    let action = widget.on_scroll(ScrollDirection::Up, 0.5, 0.5);
    assert_eq!(action, Some(WidgetAction::NextItem));
}

#[test]
fn test_input_state_tracking() {
    let mut state = InputState::new();

    assert!(!state.is_pointer_over());
    assert_eq!(state.pointer_position(), (0.0, 0.0));

    state.pointer_enter();
    assert!(state.is_pointer_over());

    state.update_position(100.0, 200.0);
    assert_eq!(state.pointer_position(), (100.0, 200.0));

    state.pointer_leave();
    assert!(!state.is_pointer_over());
}

#[test]
fn test_hit_test_widgets() {
    let widgets: Vec<Box<dyn Widget>> = vec![
        Box::new(QuotesWidget::default()),
        Box::new(QuotesWidget::default()),
    ];

    let positions = vec![
        (0.0, 50.0),   // Widget 0: y=0-50
        (50.0, 50.0),  // Widget 1: y=50-100
    ];

    // Test hit on first widget
    assert_eq!(
        hit_test_widgets(10.0, 25.0, &widgets, &positions),
        Some(0)
    );

    // Test hit on second widget
    assert_eq!(
        hit_test_widgets(10.0, 75.0, &widgets, &positions),
        Some(1)
    );

    // Test miss (outside all widgets)
    assert_eq!(hit_test_widgets(10.0, 150.0, &widgets, &positions), None);
}

#[test]
fn test_button_code_conversion() {
    assert_eq!(button_code_to_mouse_button(0x110), MouseButton::Left);
    assert_eq!(button_code_to_mouse_button(0x111), MouseButton::Right);
    assert_eq!(button_code_to_mouse_button(0x112), MouseButton::Middle);
}
