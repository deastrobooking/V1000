//! FFmpeg-backed video file decoder.
//!
//! Built only with the `ffmpeg` feature. Decodes a single video stream to
//! RGBA8 frames addressed by frame index, caching decoded frames in an
//! [`v1000_core::FrameCache`].
//!
//! Random access is sequential-with-rewind: forward requests decode forward;
//! a backward request that misses the cache rewinds to the start and decodes
//! forward to the target. Correct but not yet GOP-seek-optimized — that
//! refinement is a later milestone.

use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;

use ffmpeg::format::Pixel;
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context as Scaler, flag::Flags};
use ffmpeg::util::frame::video::Video as VideoFrame;
use ffmpeg_the_third as ffmpeg;

use v1000_core::{Frame, FrameCache};

use crate::source::{FrameSource, SourceError};

fn media_err<E: std::fmt::Display>(e: E) -> SourceError {
    SourceError::Media(e.to_string())
}

/// Decodes a video file to RGBA8 frames.
pub struct FileDecoder {
    ictx: ffmpeg::format::context::Input,
    decoder: ffmpeg::decoder::Video,
    scaler: Scaler,
    stream_index: usize,
    width: u32,
    height: u32,
    fps_num: u32,
    fps_den: u32,
    frame_count: u64,
    /// Index of the next frame `next_frame` will yield in stream order.
    cursor: u64,
    eof: bool,
    queue: VecDeque<Frame>,
    cache: FrameCache,
}

impl FileDecoder {
    /// Opens `path` and prepares its primary video stream for decoding.
    ///
    /// # Errors
    /// Returns [`SourceError::Media`] if FFmpeg cannot initialize, the file
    /// cannot be opened, or it has no decodable video stream.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SourceError> {
        ffmpeg::init().map_err(media_err)?;

        let ictx = ffmpeg::format::input(path.as_ref()).map_err(media_err)?;
        let stream = ictx
            .streams()
            .best(Type::Video)
            .ok_or_else(|| SourceError::Media("no video stream".into()))?;
        let stream_index = stream.index();

        let rate = stream.avg_frame_rate();
        let (mut fps_num, mut fps_den) = (rate.numerator(), rate.denominator());
        if fps_num <= 0 || fps_den <= 0 {
            (fps_num, fps_den) = (30, 1);
        }

        // Frame count: prefer the stream's own count, else estimate from duration.
        let stream_frames = stream.frames();
        let frame_count = if stream_frames > 0 {
            stream_frames as u64
        } else {
            let secs = ictx.duration() as f64 / f64::from(ffmpeg::ffi::AV_TIME_BASE);
            let est = secs * fps_num as f64 / fps_den as f64;
            est.max(1.0) as u64
        };

        let decoder_ctx = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
            .map_err(media_err)?;
        let decoder = decoder_ctx.decoder().video().map_err(media_err)?;

        let (width, height) = (decoder.width(), decoder.height());
        let scaler = Scaler::get(
            decoder.format(),
            width,
            height,
            Pixel::RGBA,
            width,
            height,
            Flags::BILINEAR,
        )
        .map_err(media_err)?;

        Ok(Self {
            ictx,
            decoder,
            scaler,
            stream_index,
            width,
            height,
            fps_num: fps_num as u32,
            fps_den: fps_den as u32,
            frame_count,
            cursor: 0,
            eof: false,
            queue: VecDeque::new(),
            cache: FrameCache::new(64),
        })
    }

    /// Pulls every frame the decoder currently has ready into the queue.
    fn drain_decoder(&mut self) -> Result<(), SourceError> {
        let mut decoded = VideoFrame::empty();
        while self.decoder.receive_frame(&mut decoded).is_ok() {
            let frame = self.scale_frame(&decoded)?;
            self.queue.push_back(frame);
        }
        Ok(())
    }

    /// Reads packets until at least one frame is queued or the stream ends.
    fn fill_queue(&mut self) -> Result<(), SourceError> {
        self.drain_decoder()?;
        while self.queue.is_empty() && !self.eof {
            let mut packet = ffmpeg::Packet::empty();
            match packet.read(&mut self.ictx) {
                Ok(()) => {
                    if packet.stream() == self.stream_index {
                        self.decoder.send_packet(&packet).map_err(media_err)?;
                        self.drain_decoder()?;
                    }
                }
                Err(ffmpeg::Error::Eof) => {
                    self.eof = true;
                    let _ = self.decoder.send_eof();
                    self.drain_decoder()?;
                }
                Err(e) => return Err(media_err(e)),
            }
        }
        Ok(())
    }

    fn next_frame(&mut self) -> Result<Option<Frame>, SourceError> {
        if self.queue.is_empty() {
            self.fill_queue()?;
        }
        Ok(self.queue.pop_front())
    }

    /// Seeks back to the start so frame indices line up from zero again.
    fn rewind(&mut self) -> Result<(), SourceError> {
        self.ictx.seek(0, ..).map_err(media_err)?;
        self.decoder.flush();
        self.queue.clear();
        self.eof = false;
        self.cursor = 0;
        Ok(())
    }

    /// Converts a decoded frame to a tightly packed RGBA8 [`Frame`], stripping
    /// any row padding the scaler added.
    fn scale_frame(&mut self, decoded: &VideoFrame) -> Result<Frame, SourceError> {
        let mut rgba = VideoFrame::empty();
        self.scaler.run(decoded, &mut rgba).map_err(media_err)?;

        let row_bytes = self.width as usize * Frame::BYTES_PER_PIXEL;
        let stride = rgba.stride(0);
        let src = rgba.data(0);
        let mut pixels = vec![0u8; Frame::byte_len(self.width, self.height)];
        for y in 0..self.height as usize {
            let s = y * stride;
            pixels[y * row_bytes..(y + 1) * row_bytes].copy_from_slice(&src[s..s + row_bytes]);
        }
        Ok(Frame::from_pixels(self.width, self.height, pixels))
    }
}

impl FrameSource for FileDecoder {
    fn fps(&self) -> (u32, u32) {
        (self.fps_num, self.fps_den)
    }

    fn frame_count(&self) -> u64 {
        self.frame_count
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn frame(&mut self, index: u64) -> Result<Arc<Frame>, SourceError> {
        if let Some(frame) = self.cache.get(index) {
            return Ok(frame);
        }
        if index < self.cursor {
            self.rewind()?;
        }
        while self.cursor <= index {
            match self.next_frame()? {
                Some(frame) => {
                    let at = self.cursor;
                    let shared = self.cache.insert(at, frame);
                    self.cursor += 1;
                    if at == index {
                        return Ok(shared);
                    }
                }
                None => {
                    return Err(SourceError::OutOfRange {
                        index,
                        count: self.cursor,
                    })
                }
            }
        }
        self.cache.get(index).ok_or(SourceError::OutOfRange {
            index,
            count: self.cursor,
        })
    }
}
