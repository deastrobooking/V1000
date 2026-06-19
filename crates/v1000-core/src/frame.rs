//! Decoded video frames.

/// A decoded RGBA8 video frame: tightly packed, 4 bytes per pixel, row-major,
/// top-left origin, no padding between rows.
///
/// RGBA8 is the common interchange format between decode, preview upload, and
/// (later) GPU effect input. Higher-bit-depth working formats arrive with the
/// color pipeline in a later milestone.
#[derive(Clone, PartialEq, Eq)]
pub struct Frame {
    width: u32,
    height: u32,
    /// `width * height * 4` bytes, RGBA order.
    pixels: Vec<u8>,
}

impl Frame {
    /// Bytes per pixel for the RGBA8 layout.
    pub const BYTES_PER_PIXEL: usize = 4;

    /// Creates a fully transparent (zeroed) frame of the given size.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; Self::byte_len(width, height)],
        }
    }

    /// Wraps existing RGBA8 pixel data.
    ///
    /// # Panics
    /// Panics if `pixels.len() != width * height * 4`.
    pub fn from_pixels(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        assert_eq!(
            pixels.len(),
            Self::byte_len(width, height),
            "pixel buffer length must be width * height * 4"
        );
        Self {
            width,
            height,
            pixels,
        }
    }

    /// The expected byte length for a `width`×`height` RGBA8 frame.
    pub fn byte_len(width: u32, height: u32) -> usize {
        width as usize * height as usize * Self::BYTES_PER_PIXEL
    }

    /// Frame width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Frame height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// `[width, height]`, convenient for GPU upload APIs.
    pub fn size(&self) -> [usize; 2] {
        [self.width as usize, self.height as usize]
    }

    /// The RGBA8 pixel bytes.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Mutable access to the RGBA8 pixel bytes.
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    /// Consumes the frame, returning its backing buffer so a pool can recycle it.
    pub fn into_pixels(self) -> Vec<u8> {
        self.pixels
    }
}

impl std::fmt::Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Frame")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bytes", &self.pixels.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_zeroed_and_correctly_sized() {
        let f = Frame::new(4, 2);
        assert_eq!(f.pixels().len(), 4 * 2 * 4);
        assert!(f.pixels().iter().all(|&b| b == 0));
    }

    #[test]
    #[should_panic(expected = "width * height * 4")]
    fn from_pixels_rejects_wrong_length() {
        Frame::from_pixels(2, 2, vec![0; 3]);
    }
}
