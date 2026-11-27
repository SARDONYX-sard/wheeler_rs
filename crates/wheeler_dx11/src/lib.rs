//! Direct3D 11 side of Wheeler, independent of the game:
//! - [`renderer`]: Dear ImGui DX11 backend (port of `imgui_impl_dx11.cpp`)
//! - [`texture`]: SVG rasterisation and texture upload
//!
//! `examples/preview.rs` opens a window and renders a mock wheel with this
//! crate, for visual checks without launching Skyrim.

pub mod renderer;
pub mod texture;

pub use renderer::Renderer;
