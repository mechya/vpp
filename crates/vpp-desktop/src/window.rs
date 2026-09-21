//! The desktop window: winit events in, the viewer's frames out.
//!
//! The window is frameless: the viewer draws its own bar, and says which
//! parts of the window drag and resize it (`WindowRegion`).

use std::num::NonZeroU32;
use std::process::ExitCode;
use std::rc::Rc;

use softbuffer::{Context, Surface};
use vpp_viewer::{ResizeEdge, Viewer, ViewerOptions, WindowCommand, WindowRegion};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::ModifiersState;
use winit::window::{CursorIcon, ResizeDirection, Window, WindowId};

use crate::keys::{viewer_key, viewer_modifiers};
use crate::present::copy_frame;

/// The window's first size, in CSS pixels, as the C++ viewer.
const START_SIZE: LogicalSize<f64> = LogicalSize::new(800.0, 520.0);
const DEFAULT_TITLE: &str = "VPP Viewer";

/// The desktop viewer, before and after its window opens.
pub(crate) struct DesktopApp {
    /// What the viewer is started with, until the window exists.
    options: Option<ViewerOptions>,
    address: String,
    running: Option<Running>,
    failed: bool,
}

/// An open window with its viewer.
struct Running {
    viewer: Viewer,
    surface: Surface<Rc<Window>, Rc<Window>>,
    window: Rc<Window>,
    /// Where the pointer is, in device pixels: mouse buttons do not say.
    cursor: (f32, f32),
    cursor_icon: CursorIcon,
    modifiers: ModifiersState,
    /// Whether the main button went down on the content, not to drag or resize.
    pressed_content: bool,
}

impl DesktopApp {
    pub(crate) fn new(options: ViewerOptions, address: String) -> Self {
        Self {
            options: Some(options),
            address,
            running: None,
            failed: false,
        }
    }

    /// Failure if the window could not be opened or drawn.
    pub(crate) fn exit_code(&self) -> ExitCode {
        if self.failed {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, message: String) {
        eprintln!("vpp-viewer: {message}");
        self.failed = true;
        event_loop.exit();
    }

    /// Prints the viewer's log, carries out its commands, and asks for a
    /// frame if it has a new one.
    fn after_event(&mut self, event_loop: &ActiveEventLoop) {
        let Some(running) = &mut self.running else {
            return;
        };
        for line in running.viewer.take_log() {
            println!("{line}");
        }
        for command in running.viewer.take_commands() {
            if command == WindowCommand::Close {
                event_loop.exit();
            } else {
                running.carry_out(command);
            }
        }
        if running.viewer.needs_redraw() {
            running.window.request_redraw();
        }
    }
}

impl ApplicationHandler for DesktopApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Desktop systems resume once; the options are used up then.
        let Some(options) = self.options.take() else {
            return;
        };
        match Running::open(event_loop, options, &self.address) {
            Ok(running) => {
                self.running = Some(running);
                self.after_event(event_loop);
            }
            Err(message) => self.fail(event_loop, format!("cannot open a window: {message}")),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(running) = &mut self.running else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                running.viewer.set_maximized(running.window.is_maximized());
                let scale = running.window.scale_factor() as f32;
                running.viewer.resize(size.width, size.height, scale);
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let size = running.window.inner_size();
                running
                    .viewer
                    .resize(size.width, size.height, scale_factor as f32);
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = running.present() {
                    self.fail(event_loop, format!("cannot draw the window: {e}"));
                    return;
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => running.modifiers = modifiers.state(),
            WindowEvent::CursorMoved { position, .. } => {
                running.cursor = (position.x as f32, position.y as f32);
                running.update_cursor_icon();
            }
            WindowEvent::MouseInput { state, button, .. } => running.mouse(state, button),
            WindowEvent::KeyboardInput { event, .. } => running.key(&event),
            _ => {}
        }
        self.after_event(event_loop);
    }
}

impl Running {
    /// Opens the window, then starts the viewer at its real size and scale.
    fn open(
        event_loop: &ActiveEventLoop,
        options: ViewerOptions,
        address: &str,
    ) -> Result<Self, String> {
        let attributes = Window::default_attributes()
            .with_title(DEFAULT_TITLE)
            .with_inner_size(START_SIZE)
            .with_decorations(false);
        let window = Rc::new(
            event_loop
                .create_window(attributes)
                .map_err(|e| e.to_string())?,
        );
        let context = Context::new(window.clone()).map_err(|e| e.to_string())?;
        let surface = Surface::new(&context, window.clone()).map_err(|e| e.to_string())?;
        let size = window.inner_size();
        let viewer = Viewer::new(
            options,
            address,
            size.width,
            size.height,
            window.scale_factor() as f32,
        );
        Ok(Self {
            viewer,
            surface,
            window,
            cursor: (0.0, 0.0),
            cursor_icon: CursorIcon::Default,
            modifiers: ModifiersState::empty(),
            pressed_content: false,
        })
    }

    /// Shows the viewer's current frame.
    fn present(&mut self) -> Result<(), softbuffer::SoftBufferError> {
        let frame = self.viewer.render();
        let width = NonZeroU32::new(frame.width()).expect("frames are at least 1 pixel");
        let height = NonZeroU32::new(frame.height()).expect("frames are at least 1 pixel");
        self.surface.resize(width, height)?;
        let mut buffer = self.surface.buffer_mut()?;
        copy_frame(frame, &mut buffer);
        buffer.present()
    }

    /// Does what the viewer asked of the window, except closing it.
    fn carry_out(&mut self, command: WindowCommand) {
        let window = &self.window;
        match command {
            WindowCommand::Close => {}
            WindowCommand::Minimize => window.set_minimized(true),
            WindowCommand::ToggleMaximize => window.set_maximized(!window.is_maximized()),
            WindowCommand::SetTitle(title) if title.is_empty() => window.set_title(DEFAULT_TITLE),
            WindowCommand::SetTitle(title) => window.set_title(&title),
            WindowCommand::SetResizable(resizable) => window.set_resizable(resizable),
            // A maximized window keeps filling the screen.
            WindowCommand::SetSize { .. } if window.is_maximized() => {}
            WindowCommand::SetSize { width, height } => {
                let current: LogicalSize<f64> =
                    window.inner_size().to_logical(window.scale_factor());
                let size = LogicalSize::new(
                    width.map_or(current.width, f64::from),
                    height.map_or(current.height, f64::from),
                );
                // The window may pick another size; a Resized event says which.
                let _ = window.request_inner_size(size);
            }
            WindowCommand::SetRoundedCorners(rounded) => set_rounded_corners(window, rounded),
        }
    }

    fn mouse(&mut self, state: ElementState, button: MouseButton) {
        let (x, y) = self.cursor;
        match (button, state) {
            (MouseButton::Left, ElementState::Pressed) => {
                self.pressed_content = false;
                // A drag or resize the system could not start is simply not done.
                let _ = match self.viewer.region_at(x, y) {
                    WindowRegion::Drag => self.window.drag_window(),
                    WindowRegion::Resize(edge) => self.window.drag_resize_window(direction(edge)),
                    WindowRegion::Content => {
                        self.pressed_content = true;
                        self.viewer.pointer_down(x, y);
                        Ok(())
                    }
                };
            }
            (MouseButton::Left, ElementState::Released) if self.pressed_content => {
                self.pressed_content = false;
                self.viewer.pointer_up(x, y);
            }
            (MouseButton::Back, ElementState::Pressed) => self.viewer.back(),
            (MouseButton::Forward, ElementState::Pressed) => self.viewer.forward(),
            _ => {}
        }
    }

    /// Shows resize arrows over the window's edges.
    fn update_cursor_icon(&mut self) {
        let (x, y) = self.cursor;
        let icon = match self.viewer.region_at(x, y) {
            WindowRegion::Resize(ResizeEdge::North | ResizeEdge::South) => CursorIcon::NsResize,
            WindowRegion::Resize(ResizeEdge::East | ResizeEdge::West) => CursorIcon::EwResize,
            WindowRegion::Resize(ResizeEdge::NorthEast | ResizeEdge::SouthWest) => {
                CursorIcon::NeswResize
            }
            WindowRegion::Resize(ResizeEdge::NorthWest | ResizeEdge::SouthEast) => {
                CursorIcon::NwseResize
            }
            WindowRegion::Drag | WindowRegion::Content => CursorIcon::Default,
        };
        if icon != self.cursor_icon {
            self.cursor_icon = icon;
            self.window.set_cursor(icon);
        }
    }

    /// Keys go to the viewer, then the text they type; Ctrl+V pastes from
    /// the clipboard into the address field.
    fn key(&mut self, event: &KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }
        // Holding a key down repeats it only while typing an address.
        if event.repeat && !self.viewer.is_editing_address() {
            return;
        }
        let modifiers = viewer_modifiers(self.modifiers);
        let Some(key) = viewer_key(event) else {
            return;
        };
        let paste = modifiers.ctrl
            && matches!(&key, vpp_viewer::Key::Character(c) if c.eq_ignore_ascii_case("v"));
        if paste && self.viewer.is_editing_address() {
            if let Some(text) = vpp_platform::clipboard_text() {
                self.viewer.paste(&text);
            }
            return;
        }
        self.viewer.key_down(key, modifiers);
        if let Some(text) = &event.text {
            self.viewer.text_input(text);
        }
    }
}

fn direction(edge: ResizeEdge) -> ResizeDirection {
    match edge {
        ResizeEdge::North => ResizeDirection::North,
        ResizeEdge::South => ResizeDirection::South,
        ResizeEdge::East => ResizeDirection::East,
        ResizeEdge::West => ResizeDirection::West,
        ResizeEdge::NorthEast => ResizeDirection::NorthEast,
        ResizeEdge::NorthWest => ResizeDirection::NorthWest,
        ResizeEdge::SouthEast => ResizeDirection::SouthEast,
        ResizeEdge::SouthWest => ResizeDirection::SouthWest,
    }
}

/// Windows 11 rounds the corners itself, at its own radius; Windows 10 cannot.
#[cfg(target_os = "windows")]
fn set_rounded_corners(window: &Window, rounded: bool) {
    use winit::platform::windows::{CornerPreference, WindowExtWindows};
    window.set_corner_preference(if rounded {
        CornerPreference::Round
    } else {
        CornerPreference::DoNotRound
    });
}

/// Elsewhere, corners stay square until each platform's step (`docs/rust-port.md` §8).
#[cfg(not(target_os = "windows"))]
fn set_rounded_corners(_: &Window, _: bool) {}
