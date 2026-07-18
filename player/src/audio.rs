use std::{fs::File, path::Path};

use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};

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

        let file = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
        let source =
            Decoder::try_from(file).with_context(|| format!("cannot decode {}", path.display()))?;
        let sink = Sink::connect_new(self.stream.mixer());
        sink.set_volume(self.volume);
        sink.append(source);
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
