//! End-to-end check that the preview reads from the *timeline*: opens a file,
//! wraps it in a single-clip `Sequence`, and pulls frames by timeline time.
//! Build with the `ffmpeg` feature.
//!
//! ```text
//! cargo run -p v1000-timeline --features ffmpeg --example sequence_probe -- video.mp4
//! ```

#[cfg(feature = "ffmpeg")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use v1000_codec::{FileDecoder, FrameProducer, FrameSource};
    use v1000_core::Time;
    use v1000_timeline::Sequence;

    let path = std::env::args()
        .nth(1)
        .ok_or("usage: sequence_probe <video>")?;

    let decoder = FileDecoder::open(&path)?;
    let timebase = {
        let (n, d) = decoder.fps();
        v1000_core::Rational::new(n as i64, d as i64)
    };
    let mut seq = Sequence::single(Box::new(decoder));
    println!(
        "sequence {:?}: {:?}, duration {:.2}s, timebase {:.3} fps",
        seq.name,
        seq.size(),
        seq.duration().as_seconds_f64(),
        timebase.to_f64()
    );

    for frame_idx in [0i64, 10, 1] {
        let t = Time::from_frame(frame_idx, timebase);
        match seq.frame_at(t)? {
            Some(frame) => println!(
                "t=frame {frame_idx}: got {}x{} ({} bytes)",
                frame.width(),
                frame.height(),
                frame.pixels().len()
            ),
            None => println!("t=frame {frame_idx}: gap"),
        }
    }

    // A time past the end must be a gap.
    let past = Time::from_frame(1_000_000, timebase);
    assert!(seq.frame_at(past)?.is_none(), "past-end must be a gap");
    println!("OK");
    Ok(())
}

#[cfg(not(feature = "ffmpeg"))]
fn main() {
    eprintln!("rebuild with --features ffmpeg");
}
