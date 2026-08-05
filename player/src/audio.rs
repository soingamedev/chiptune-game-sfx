use std::{ffi::OsStr, fs::File, path::Path};

use anyhow::{anyhow, Context, Result};
use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
use xmrs::prelude::Module;
use xmrsplayer::xmrsplayer::XmrsPlayer;

/// Tracker-module extensions rendered via `xmrs`/`xmrsplayer` (everything else
/// goes through rodio's `Decoder`, i.e. OGG/WAV/...).
const TRACKER_EXTS: &[&str] = &["xm", "mod", "s3m", "it"];

/// Output rate for tracker rendering; rodio resamples to the device as needed.
const TRACKER_RATE: u32 = 48_000;
/// Safety ceiling on a single rendered pass (stereo samples): 120 s. Our loops
/// are a few seconds, so this only guards against a pathological module.
const TRACKER_SAMPLE_CAP: usize = (TRACKER_RATE as usize) * 2 * 120;

/// True when `path` is a tracker module we render ourselves.
pub fn is_tracker(path: &Path) -> bool {
    match path.extension().and_then(OsStr::to_str) {
        Some(ext) => {
            let ext = ext.to_ascii_lowercase();
            TRACKER_EXTS.contains(&ext.as_str())
        }
        None => false,
    }
}

/// Render a tracker module (XM/MOD/S3M/IT) to a rodio `SamplesBuffer`.
///
/// `xmrsplayer` is the same `xmrs` lineage the GBA build's `agb_tracker` uses, so
/// what plays here is a faithful desktop preview of the on-device music. We
/// PRE-RENDER a single pass into an owned buffer (module + player are local and
/// dropped at return) — this sidesteps `XmrsPlayer`'s `&Module` borrow without a
/// self-referential struct or a leak, and lets rodio own/loop the result.
fn render_tracker(path: &Path) -> Result<SamplesBuffer> {
    let bytes =
        std::fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
    let module =
        Module::load(&bytes).map_err(|e| anyhow!("cannot import {}: {e:?}", path.display()))?;

    // Sub-song 0 (XM/MOD/S3M/IT expose one); one loop pass, from the top.
    let mut player = XmrsPlayer::new(&module, TRACKER_RATE, 0);
    player.set_max_loop_count(1);
    player.goto(0, 0, 0);

    // The player yields interleaved stereo `i16`; convert to rodio's `f32`.
    let mut buf: Vec<f32> = Vec::new();
    for v in player.by_ref() {
        buf.push(v as f32 / i16::MAX as f32);
        if buf.len() >= TRACKER_SAMPLE_CAP {
            break;
        }
    }
    if buf.len() % 2 == 1 {
        buf.pop(); // keep whole stereo frames
    }
    if buf.is_empty() {
        return Err(anyhow!("no audio rendered from {}", path.display()));
    }
    Ok(SamplesBuffer::new(2, TRACKER_RATE, buf))
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
    Finished,
}

pub struct AudioPlayer {
    stream: OutputStream,
    sink: Option<Sink>,
    volume: f32,
    paused: bool,
}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        let stream = OutputStreamBuilder::open_default_stream()?;

        Ok(Self {
            stream,
            sink: None,
            volume: 0.8,
            paused: false,
        })
    }

    pub fn play(&mut self, path: &Path) -> Result<()> {
        self.stop();

        let sink = Sink::connect_new(self.stream.mixer());
        sink.set_volume(self.volume);

        if is_tracker(path) {
            // Tracker music: pre-render one pass, then loop it for the preview.
            let source = render_tracker(path)?;
            sink.append(source.repeat_infinite());
        } else {
            // OGG / WAV / ... via rodio's built-in decoder (one-shot).
            let file =
                File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
            let source = Decoder::try_from(file)
                .with_context(|| format!("cannot decode {}", path.display()))?;
            sink.append(source);
        }
        sink.play();

        self.sink = Some(sink);
        self.paused = false;
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.paused = false;
    }

    pub fn toggle_pause(&mut self) {
        let Some(sink) = self.sink.as_ref() else {
            return;
        };

        if self.paused {
            sink.play();
            self.paused = false;
        } else {
            sink.pause();
            self.paused = true;
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);

        if let Some(sink) = self.sink.as_ref() {
            sink.set_volume(self.volume);
        }
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn state(&self) -> PlaybackState {
        match self.sink.as_ref() {
            None => PlaybackState::Stopped,
            Some(sink) if sink.empty() => PlaybackState::Finished,
            Some(_) if self.paused => PlaybackState::Paused,
            Some(_) => PlaybackState::Playing,
        }
    }
}
