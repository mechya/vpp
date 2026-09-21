//! Copying a frame into the window's pixel buffer.

use vpp_viewer::Pixmap;

/// Converts `frame` (RGBA bytes, premultiplied) into softbuffer's format:
/// one `u32` per pixel, `0x00RRGGBB`. Frames are opaque, so premultiplied
/// and straight colour are the same.
pub(crate) fn copy_frame(frame: &Pixmap, buffer: &mut [u32]) {
    for (out, px) in buffer.iter_mut().zip(frame.data().chunks_exact(4)) {
        *out = u32::from(px[0]) << 16 | u32::from(px[1]) << 8 | u32::from(px[2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_becomes_0rgb() {
        let mut frame = Pixmap::new(2, 1).unwrap();
        frame
            .data_mut()
            .copy_from_slice(&[0x12, 0x34, 0x56, 0xff, 0xff, 0x00, 0x80, 0xff]);
        let mut buffer = [0u32; 2];
        copy_frame(&frame, &mut buffer);
        assert_eq!(buffer, [0x0012_3456, 0x00ff_0080]);
    }
}
