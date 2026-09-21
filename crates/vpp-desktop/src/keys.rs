//! Translating winit's keyboard events into the viewer's keys.

use vpp_viewer::{Key, Modifiers};
use winit::event::KeyEvent;
use winit::keyboard::{Key as WinitKey, ModifiersState, NamedKey};
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;

/// The viewer's key for `event`, if it is one the viewer knows. Letters are
/// read without modifiers, so Ctrl+L is `"l"` with Ctrl held.
pub(crate) fn viewer_key(event: &KeyEvent) -> Option<Key> {
    Some(match event.key_without_modifiers() {
        WinitKey::Named(named) => match named {
            NamedKey::Escape => Key::Escape,
            NamedKey::Enter => Key::Enter,
            NamedKey::Backspace => Key::Backspace,
            NamedKey::F5 => Key::F5,
            NamedKey::ArrowLeft => Key::ArrowLeft,
            NamedKey::ArrowRight => Key::ArrowRight,
            NamedKey::BrowserBack => Key::BrowserBack,
            NamedKey::BrowserForward => Key::BrowserForward,
            NamedKey::BrowserRefresh => Key::BrowserRefresh,
            _ => return None,
        },
        WinitKey::Character(c) => Key::Character(c.to_string()),
        _ => return None,
    })
}

pub(crate) fn viewer_modifiers(state: ModifiersState) -> Modifiers {
    Modifiers {
        ctrl: state.control_key(),
        alt: state.alt_key(),
        shift: state.shift_key(),
    }
}
