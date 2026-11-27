//! SVG icon loading (C++ `Texture::load_texture_from_file`, which used
//! nanosvg): rasterise at native size (96 dpi, scale 1), then upload.

use std::path::Path;

use rex_com::ID3D11Device;

/// Straight (non-premultiplied) RGBA8 pixels, like nanosvg produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// An uploaded texture: `srv` is the `ID3D11ShaderResourceView*` (also the
/// ImGui texture id).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GpuTexture {
    pub srv: usize,
    pub width: i32,
    pub height: i32,
}

/// Rasterises SVG data at its intrinsic size.
pub fn rasterize_svg(data: &[u8]) -> Result<RgbaImage, String> {
    let tree = resvg::usvg::Tree::from_data(data, &resvg::usvg::Options::default())
        .map_err(|e| e.to_string())?;
    let size = tree.size().to_int_size();
    let (width, height) = (size.width(), size.height());
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).ok_or("empty SVG")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for p in pixmap.pixels() {
        let c = p.demultiply();
        pixels.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    Ok(RgbaImage {
        width,
        height,
        pixels,
    })
}

pub fn rasterize_svg_file(path: &Path) -> Result<RgbaImage, String> {
    let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    rasterize_svg(&data).map_err(|e| format!("{}: {e}", path.display()))
}

/// Uploads `image` as an immutable texture.
///
/// # Safety
/// `device` must be a live D3D11 device.
pub unsafe fn upload(device: *mut ID3D11Device, image: &RgbaImage) -> Result<GpuTexture, String> {
    let srv = unsafe {
        crate::renderer::create_texture_rgba(device, &image.pixels, image.width, image.height)?
    };
    Ok(GpuTexture {
        srv: srv as usize,
        width: image.width as i32,
        height: image.height as i32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterizes_a_square() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="2"><rect width="4" height="2" fill="#ff0000"/></svg>"##;
        let image = rasterize_svg(svg).unwrap();
        assert_eq!((image.width, image.height), (4, 2));
        assert_eq!(&image.pixels[..4], &[255, 0, 0, 255]);
    }
}
