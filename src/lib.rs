//! The purpose of this library is to provide an OpenGL context on as many
//!  platforms as possible.
//!
//! # Building a window
//!
//! There are two ways to create a window:
//!
//!  - Calling `Window::new()`.
//!  - Calling `let builder = WindowBuilder::new()` then `builder.build()`.
//!
//! The first way is the simpliest way and will give you default values.
//!
//! The second way allows you to customize the way your window and GL context
//!  will look and behave.
//!
//! # Features
//!
//! This crate has two Cargo features: `window` and `headless`.
//!
//!  - `window` allows you to create regular windows and enables the `WindowBuilder` object.
//!  - `headless` allows you to do headless rendering, and enables
//!     the `HeadlessRendererBuilder` object.
//!
//! By default only `window` is enabled.

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate shared_library;

extern crate libc;

#[cfg(target_os = "windows")]
extern crate winapi;
#[cfg(target_os = "windows")]
extern crate kernel32;
#[cfg(target_os = "windows")]
extern crate shell32;
#[cfg(target_os = "windows")]
extern crate gdi32;
#[cfg(target_os = "windows")]
extern crate user32;
#[cfg(target_os = "windows")]
extern crate dwmapi;
#[cfg(any(target_os = "macos", target_os = "ios"))]
#[macro_use]
extern crate objc;
#[cfg(target_os = "macos")]
extern crate cgl;
#[cfg(target_os = "macos")]
extern crate cocoa;
#[cfg(target_os = "macos")]
extern crate core_foundation;
#[cfg(target_os = "macos")]
extern crate core_graphics;
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "dragonfly"))]
extern crate x11_dl;

pub use events::*;
pub use headless::{HeadlessRendererBuilder, HeadlessContext};
pub use window::{WindowBuilder, WindowProxy, PollEventsIterator, WaitEventsIterator};
pub use window::{AvailableMonitorsIter, MonitorId, get_available_monitors, get_primary_monitor};
pub use native_monitor::NativeMonitorId;

use std::io;
#[cfg(not(target_os = "macos"))]
use std::cmp::Ordering;

mod api;
mod platform;
mod events;
mod headless;
mod window;

pub mod os;

/// Represents an OpenGL context and the Window or environment around it.
///
/// # Example
///
/// ```ignore
/// let window = Window::new().unwrap();
///
/// unsafe { window.make_current() };
///
/// loop {
///     for event in window.poll_events() {
///         match(event) {
///             // process events here
///             _ => ()
///         }
///     }
///
///     // draw everything here
///
///     window.swap_buffers();
///     std::old_io::timer::sleep(17);
/// }
/// ```
pub struct Window {
    window: platform::Window,
}

/// Trait that describes objects that have access to an OpenGL context.
pub trait GlContext {
    /// Sets the context as the current context.
    unsafe fn make_current(&self) -> Result<(), ContextError>;

    /// Returns true if this context is the current one in this thread.
    fn is_current(&self) -> bool;

    /// Returns the address of an OpenGL function.
    fn get_proc_address(&self, addr: &str) -> *const ();

    /// Swaps the buffers in case of double or triple buffering.
    ///
    /// You should call this function every time you have finished rendering, or the image
    /// may not be displayed on the screen.
    ///
    /// **Warning**: if you enabled vsync, this function will block until the next time the screen
    /// is refreshed. However drivers can choose to override your vsync settings, which means that
    /// you can't know in advance whether `swap_buffers` will block or not.
    fn swap_buffers(&self) -> Result<(), ContextError>;

    /// Returns the OpenGL API being used.
    fn get_api(&self) -> Api;

    /// Returns the pixel format of the main framebuffer of the context.
    fn get_pixel_format(&self) -> PixelFormat;
}

/// Error that can happen while creating a window or a headless renderer.
#[derive(Debug)]
pub enum CreationError {
    OsError(String),
    /// TODO: remove this error
    NotSupported,
    NoBackendAvailable(Box<std::error::Error + Send>),
    RobustnessNotSupported,
    OpenGlVersionNotSupported,
    NoAvailablePixelFormat,
}

impl CreationError {
    fn to_string(&self) -> &str {
        match *self {
            CreationError::OsError(ref text) => &text,
            CreationError::NotSupported => "Some of the requested attributes are not supported",
            CreationError::NoBackendAvailable(_) => "No backend is available",
            CreationError::RobustnessNotSupported => "You requested robustness, but it is \
                                                      not supported.",
            CreationError::OpenGlVersionNotSupported => "The requested OpenGL version is not \
                                                         supported.",
            CreationError::NoAvailablePixelFormat => "Couldn't find any pixel format that matches \
                                                      the criterias.",
        }
    }
}

impl std::fmt::Display for CreationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        formatter.write_str(self.to_string())
    }
}

impl std::error::Error for CreationError {
    fn description(&self) -> &str {
        self.to_string()
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            CreationError::NoBackendAvailable(ref err) => Some(&**err),
            _ => None
        }
    }
}

/// Error that can happen when manipulating an OpenGL context.
#[derive(Debug)]
pub enum ContextError {
    IoError(io::Error),
    ContextLost,
}

impl ContextError {
    fn to_string(&self) -> &str {
        use std::error::Error;
        match *self {
            ContextError::IoError(ref err) => err.description(),
            ContextError::ContextLost => "Context lost"
        }
    }
}

impl std::fmt::Display for ContextError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        formatter.write_str(self.to_string())
    }
}

impl std::error::Error for ContextError {
    fn description(&self) -> &str {
        self.to_string()
    }
}

/// All APIs related to OpenGL that you can possibly get while using glutin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Api {
    /// The classical OpenGL. Available on Windows, Linux, OS/X.
    OpenGl,
    /// OpenGL embedded system. Available on Linux, Android.
    OpenGlEs,
    /// OpenGL for the web. Very similar to OpenGL ES.
    WebGl,
}

/// Describes the requested OpenGL context profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlProfile {
    /// Include all the immediate more functions and definitions.
    Compatibility,
    /// Include all the future-compatible functions and definitions.
    Core,
}

/// Describes the OpenGL API and version that are being requested when a context is created.
#[derive(Debug, Copy, Clone)]
pub enum GlRequest {
    /// Request the latest version of the "best" API of this platform.
    ///
    /// On desktop, will try OpenGL.
    Latest,

    /// Request a specific version of a specific API.
    ///
    /// Example: `GlRequest::Specific(Api::OpenGl, (3, 3))`.
    Specific(Api, (u8, u8)),

    /// If OpenGL is available, create an OpenGL context with the specified `opengl_version`.
    /// Else if OpenGL ES or WebGL is available, create a context with the
    /// specified `opengles_version`.
    GlThenGles {
        /// The version to use for OpenGL.
        opengl_version: (u8, u8),
        /// The version to use for OpenGL ES.
        opengles_version: (u8, u8),
    },
}

impl GlRequest {
    /// Extract the desktop GL version, if any.
    pub fn to_gl_version(&self) -> Option<(u8, u8)> {
        match self {
            &GlRequest::Specific(Api::OpenGl, version) => Some(version),
            &GlRequest::GlThenGles { opengl_version: version, .. } => Some(version),
            _ => None,
        }
    }
}

/// The minimum core profile GL context. Useful for getting the minimum
/// required GL version while still running on OSX, which often forbids
/// the compatibility profile features.
pub static GL_CORE: GlRequest = GlRequest::Specific(Api::OpenGl, (3, 2));

/// Specifies the tolerance of the OpenGL context to faults. If you accept raw OpenGL commands
/// and/or raw shader code from an untrusted source, you should definitely care about this.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Robustness {
    /// Not everything is checked. Your application can crash if you do something wrong with your
    /// shaders.
    NotRobust,

    /// The driver doesn't check anything. This option is very dangerous. Please know what you're
    /// doing before using it. See the `GL_KHR_no_error` extension.
    ///
    /// Since this option is purely an optimisation, no error will be returned if the backend
    /// doesn't support it. Instead it will automatically fall back to `NotRobust`.
    NoError,

    /// Everything is checked to avoid any crash. The driver will attempt to avoid any problem,
    /// but if a problem occurs the behavior is implementation-defined. You are just guaranteed not
    /// to get a crash.
    RobustNoResetNotification,

    /// Same as `RobustNoResetNotification` but the context creation doesn't fail if it's not
    /// supported.
    TryRobustNoResetNotification,

    /// Everything is checked to avoid any crash. If a problem occurs, the context will enter a
    /// "context lost" state. It must then be recreated. For the moment, glutin doesn't provide a
    /// way to recreate a context with the same window :-/
    RobustLoseContextOnReset,

    /// Same as `RobustLoseContextOnReset` but the context creation doesn't fail if it's not
    /// supported.
    TryRobustLoseContextOnReset,
}

#[derive(Debug, Copy, Clone)]
pub enum MouseCursor {
    /// The platform-dependent default cursor.
    Default,
    /// A simple crosshair.
    Crosshair,
    /// A hand (often used to indicate links in web browsers).
    Hand,
    /// Self explanatory.
    Arrow,
    /// Indicates something is to be moved.
    Move,
    /// Indicates text that may be selected or edited.
    Text,
    /// Program busy indicator.
    Wait,
    /// Help indicator (often rendered as a "?")
    Help,
    /// Progress indicator. Shows that processing is being done. But in contrast
    /// with "Wait" the user may still interact with the program. Often rendered
    /// as a spinning beach ball, or an arrow with a watch or hourglass.
    Progress,

    /// Cursor showing that something cannot be done.
    NotAllowed,
    ContextMenu,
    NoneCursor,
    Cell,
    VerticalText,
    Alias,
    Copy,
    NoDrop,
    Grab,
    Grabbing,
    AllScroll,
    ZoomIn,
    ZoomOut,

    /// Indicate that some edge is to be moved. For example, the 'SeResize' cursor
    /// is used when the movement starts from the south-east corner of the box.
    EResize,
    NResize,
    NeResize,
    NwResize,
    SResize,
    SeResize,
    SwResize,
    WResize,
    EwResize,
    NsResize,
    NeswResize,
    NwseResize,
    ColResize,
    RowResize,
}

/// Describes how glutin handles the cursor.
#[derive(Debug, Copy, Clone)]
pub enum CursorState {
    /// Normal cursor behavior.
    Normal,

    /// The cursor will be invisible when over the window.
    Hide,

    /// Grabs the mouse cursor. The cursor's motion will be confined to this
    /// window and the window has exclusive access to further events regarding
    /// the cursor.
    ///
    /// This is useful for first-person cameras for example.
    Grab,
}

/// Describes a possible format. Unused.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct PixelFormat {
    pub hardware_accelerated: bool,
    pub color_bits: u8,
    pub alpha_bits: u8,
    pub depth_bits: u8,
    pub stencil_bits: u8,
    pub stereoscopy: bool,
    pub double_buffer: bool,
    pub multisampling: Option<u16>,
    pub srgb: bool,
}

/// VERY UNSTABLE! Describes how the backend should choose a pixel format.
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct PixelFormatRequirements {
    pub multisampling: Option<u16>,
    pub depth_bits: Option<u8>,
    pub stencil_bits: Option<u8>,
    pub color_bits: Option<u8>,
    pub alpha_bits: Option<u8>,
    pub stereoscopy: bool,
    pub srgb: Option<bool>,
}

impl PixelFormatRequirements {
    #[cfg(not(target_os = "macos"))]
    fn choose_pixel_format<T, I>(&self, iter: I) -> Result<(T, PixelFormat), CreationError>
                                 where I: IntoIterator<Item=(T, PixelFormat)>, T: Clone
    {
        // filtering formats that don't match the requirements
        let iter = iter.into_iter().filter(|&(_, ref format)| {
            if format.color_bits < self.color_bits.unwrap_or(0) {
                return false;
            }

            if format.alpha_bits < self.alpha_bits.unwrap_or(0) {
                return false;
            }

            if format.depth_bits < self.depth_bits.unwrap_or(0) {
                return false;
            }

            if format.stencil_bits < self.stencil_bits.unwrap_or(0) {
                return false;
            }

            if !format.stereoscopy && self.stereoscopy {
                return false;
            }

            if let Some(req_ms) = self.multisampling {
                match format.multisampling {
                    Some(val) if val >= req_ms => (),
                    _ => return false
                }
            } else {
                if format.multisampling.is_some() {
                    return false;
                }
            }

            if let Some(srgb) = self.srgb {
                if srgb != format.srgb {
                    return false;
                }
            }

            true
        });

        // sorting so that the preferred format comes first
        let mut formats = iter.collect::<Vec<_>>();
        formats.sort_by(|&(_, ref left), &(_, ref right)| {
            // prefer hardware-accelerated formats
            if left.hardware_accelerated && !right.hardware_accelerated {
                return Ordering::Less;
            } else if right.hardware_accelerated && !left.hardware_accelerated {
                return Ordering::Greater;
            }

            // prefer sRGB formats
            if left.srgb && !right.srgb {
                return Ordering::Less;
            } else if right.srgb && !left.srgb {
                return Ordering::Greater;
            }

            // prefer formats with the highest color+alpha bits
            if left.color_bits + left.alpha_bits != right.color_bits + right.alpha_bits {
                return (right.color_bits + right.alpha_bits)
                                                        .cmp(&(left.color_bits + left.alpha_bits));
            }

            // prefer double-buffering formats
            if left.double_buffer && !right.double_buffer {
                return Ordering::Less;
            } else if right.double_buffer && !left.double_buffer {
                return Ordering::Greater;
            }

            // prefer formats with the highest depth bits
            if left.depth_bits != right.depth_bits {
                return (right.depth_bits).cmp(&left.depth_bits);
            }

            // prefer formats with the highest stencil bits
            if left.stencil_bits != right.stencil_bits {
                return (right.stencil_bits).cmp(&left.stencil_bits);
            }

            // prefer formats with multisampling
            if left.multisampling.is_some() && right.multisampling.is_none() {
                return Ordering::Less;
            } else if right.multisampling.is_some() && left.multisampling.is_none() {
                return Ordering::Greater;
            }

            // default
            return Ordering::Equal;
        });

        formats.into_iter().next().ok_or(CreationError::NoAvailablePixelFormat)
    }
}

impl Default for PixelFormatRequirements {
    #[inline]
    fn default() -> PixelFormatRequirements {
        PixelFormatRequirements {
            multisampling: None,
            depth_bits: None,
            stencil_bits: None,
            color_bits: None,
            alpha_bits: None,
            stereoscopy: false,
            srgb: None,
        }
    }
}

/// Attributes to use when creating a window.
#[derive(Clone)]
pub struct WindowAttributes {
    /// The dimensions of the window. If this is `None`, some platform-specific dimensions will be
    /// used.
    ///
    /// The default is `None`.
    pub dimensions: Option<(u32, u32)>,

    /// If `Some`, the window will be in fullscreen mode with the given monitor.
    ///
    /// The default is `None`.
    pub monitor: Option<platform::MonitorId>,

    /// The title of the window in the title bar.
    ///
    /// The default is `"glutin window"`.
    pub title: String,

    /// Whether the window should be immediately visible upon creation.
    ///
    /// The default is `true`.
    pub visible: bool,

    /// Whether the the window should be transparent. If this is true, writing colors
    /// with alpha values different than `1.0` will produce a transparent window.
    ///
    /// The default is `false`.
    pub transparent: bool,

    /// Whether the window should have borders and bars.
    ///
    /// The default is `true`.
    pub decorations: bool,

    /// [iOS only] Enable multitouch, see [UIView#multipleTouchEnabled]
    /// (https://developer.apple.com/library/ios/documentation/UIKit/Reference/UIView_Class/#//apple_ref/occ/instp/UIView/multipleTouchEnabled)
    pub multitouch: bool,

    pub root_accessible: Option<platform::AccessibleRef>,
}

impl Default for WindowAttributes {
    #[inline]
    fn default() -> WindowAttributes {
        WindowAttributes {
            dimensions: None,
            monitor: None,
            title: "glutin window".to_owned(),
            visible: true,
            transparent: false,
            decorations: true,
            multitouch: false,
            root_accessible: None,
        }
    }
}

/// Attributes to use when creating an OpenGL context.
#[derive(Clone)]
pub struct GlAttributes<S> {
    /// An existing context to share the new the context with.
    ///
    /// The default is `None`.
    pub sharing: Option<S>,

    /// Version to try create. See `GlRequest` for more infos.
    ///
    /// The default is `Latest`.
    pub version: GlRequest,

    /// OpenGL profile to use.
    ///
    /// The default is `None`.
    pub profile: Option<GlProfile>,

    /// Whether to enable the `debug` flag of the context.
    ///
    /// Debug contexts are usually slower but give better error reporting.
    ///
    /// The default is `true` in debug mode and `false` in release mode.
    pub debug: bool,

    /// How the OpenGL context should detect errors.
    ///
    /// The default is `NotRobust` because this is what is typically expected when you create an
    /// OpenGL context. However for safety you should consider `TryRobustLoseContextOnReset`.
    pub robustness: Robustness,

    /// Whether to use vsync. If vsync is enabled, calling `swap_buffers` will block until the
    /// screen refreshes. This is typically used to prevent screen tearing.
    ///
    /// The default is `false`.
    pub vsync: bool,
}

impl<S> GlAttributes<S> {
    /// Turns the `sharing` parameter into another type by calling a closure.
    #[inline]
    pub fn map_sharing<F, T>(self, f: F) -> GlAttributes<T> where F: FnOnce(S) -> T {
        GlAttributes {
            sharing: self.sharing.map(f),
            version: self.version,
            profile: self.profile,
            debug: self.debug,
            robustness: self.robustness,
            vsync: self.vsync,
        }
    }
}

impl<S> Default for GlAttributes<S> {
    #[inline]
    fn default() -> GlAttributes<S> {
        GlAttributes {
            sharing: None,
            version: GlRequest::Latest,
            profile: None,
            debug: cfg!(debug_assertions),
            robustness: Robustness::NotRobust,
            vsync: false,
        }
    }
}

mod native_monitor {
    /// Native platform identifier for a monitor. Different platforms use fundamentally different types
    /// to represent a monitor ID.
    #[derive(Clone, PartialEq, Eq)]
    pub enum NativeMonitorId {
        /// Cocoa and X11 use a numeric identifier to represent a monitor.
        Numeric(u32),

        /// Win32 uses a Unicode string to represent a monitor.
        Name(String),

        /// Other platforms (Android) don't support monitor identification.
        Unavailable
    }
}
