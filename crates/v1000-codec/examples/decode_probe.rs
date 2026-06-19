//! Decode-path smoke check: opens a video file, decodes a few frames, and
//! prints what it found. Build with the `ffmpeg` feature.
//!
//! ```text
//! cargo run -p v1000-codec --features ffmpeg --example decode_probe -- /path/to/video.mp4
//! ```

#[cfg(feature = "ffmpeg")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use v1000_codec::{FileDecoder, FrameSource};

    let path = std::env::args()
        .nth(1)
        .ok_or("usage: decode_probe <video>")?;
    let mut dec = FileDecoder::open(&path)?;

    let (w, h) = dec.dimensions();
    let (fn_, fd) = dec.fps();
    println!(
        "opened {path}: {w}x{h}, {:.3} fps, {} frames, {:.2}s",
        fn_ as f64 / fd as f64,
        dec.frame_count(),
        dec.duration_seconds()
    );

    // Decode frame 0, a middle frame, then re-request frame 0 (exercises rewind).
    for idx in [0u64, dec.frame_count() / 2, 0] {
        let frame = dec.frame(idx)?;
        let p = frame.pixels();
        println!(
            "frame {idx}: {}x{}, {} bytes, first px rgba={:?}",
            frame.width(),
            frame.height(),
            p.len(),
            &p[0..4]
        );
        assert_eq!(
            p.len(),
            frame.width() as usize * frame.height() as usize * 4
        );
    }
    println!("OK");
    Ok(())
}

#[cfg(not(feature = "ffmpeg"))]
fn main() {
    eprintln!("rebuild with --features ffmpeg");
}
