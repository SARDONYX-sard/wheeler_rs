//! Wheeler drawing on top of Dear ImGui (game independent): primitives,
//! images, and the wheel / entry logic with items abstracted as traits.
//!
//! [`headless`] renders frames without a GPU so drawing code can be tested.

pub mod drawer;
pub mod edit_banner;
pub mod headless;
pub mod image;
pub mod settings;
pub mod wheel;

#[cfg(test)]
mod tests;
