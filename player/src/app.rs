use std::path::PathBuf;

use anyhow::Result;

use crate::{
    audio::{AudioPlayer, PlaybackState},
    library::{AudioLibrary, Track},
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Focus {
    Categories,
    Tracks,
}

#[derive(Debug, Clone)]
pub struct TrackView {
    pub category_index: usize,
    pub track_index: usize,
    pub label: String,
}

pub struct App {
    pub library: AudioLibrary,
    pub audio_root: PathBuf,
    pub audio: AudioPlayer,
    pub focus: Focus,
    pub selected_category: usize,
    pub selected_track: usize,
    pub search_active: bool,
    pub search_query: String,
    pub should_quit: bool,
    pub now_playing: Option<String>,
    pub message: String,
    pub playback_state: PlaybackState,
}

impl App {
    pub fn new(library: AudioLibrary, audio_root: PathBuf, audio: AudioPlayer) -> Self {
        let total_tracks = library.total_tracks;
        let category_count = library.categories.len();

        Self {
            library,
            audio_root,
            audio,
            focus: Focus::Tracks,
            selected_category: 0,
            selected_track: 0,
            search_active: false,
            search_query: String::new(),
            should_quit: false,
            now_playing: None,
            message: format!("Loaded {total_tracks} sounds in {category_count} categories"),
            playback_state: PlaybackState::Stopped,
        }
    }

    pub fn visible_tracks(&self) -> Vec<TrackView> {
        if self.search_query.trim().is_empty() {
            return self
                .library
                .categories
                .get(self.selected_category)
                .map(|category| {
                    category
                        .tracks
                        .iter()
                        .enumerate()
                        .map(|(track_index, track)| TrackView {
                            category_index: self.selected_category,
                            track_index,
                            label: track.display_name.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();
        }

        let query = self.search_query.to_lowercase();
        let mut matches = Vec::new();

        for (category_index, category) in self.library.categories.iter().enumerate() {
            for (track_index, track) in category.tracks.iter().enumerate() {
                let haystack = format!(
                    "{} {} {}",
                    category.name, track.display_name, track.file_name
                )
                .to_lowercase();

                if haystack.contains(&query) {
                    matches.push(TrackView {
                        category_index,
                        track_index,
                        label: format!("{} / {}", category.name, track.display_name),
                    });
                }
            }
        }

        matches
    }

    pub fn selected_track_ref(&self) -> Option<&Track> {
        let visible = self.visible_tracks();
        let selected = visible.get(self.selected_track)?;

        self.library
            .categories
            .get(selected.category_index)?
            .tracks
            .get(selected.track_index)
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Categories => Focus::Tracks,
            Focus::Tracks => Focus::Categories,
        };
    }

    pub fn previous_item(&mut self) {
        match self.focus {
            Focus::Categories => self.previous_category(),
            Focus::Tracks => self.previous_track(),
        }
    }

    pub fn next_item(&mut self) {
        match self.focus {
            Focus::Categories => self.next_category(),
            Focus::Tracks => self.next_track(),
        }
    }

    pub fn previous_category(&mut self) {
        if self.library.categories.is_empty() {
            return;
        }

        self.selected_category = self
            .selected_category
            .checked_sub(1)
            .unwrap_or_else(|| self.library.categories.len() - 1);
        self.selected_track = 0;
        self.search_query.clear();
    }

    pub fn next_category(&mut self) {
        if self.library.categories.is_empty() {
            return;
        }

        self.selected_category = (self.selected_category + 1) % self.library.categories.len();
        self.selected_track = 0;
        self.search_query.clear();
    }

    pub fn previous_track(&mut self) {
        let track_count = self.visible_tracks().len();
        if track_count == 0 {
            self.selected_track = 0;
            return;
        }

        self.selected_track = self
            .selected_track
            .checked_sub(1)
            .unwrap_or(track_count.saturating_sub(1));
    }

    pub fn next_track(&mut self) {
        let track_count = self.visible_tracks().len();
        if track_count == 0 {
            self.selected_track = 0;
            return;
        }

        self.selected_track = (self.selected_track + 1) % track_count;
    }

    pub fn enter_search(&mut self) {
        self.search_active = true;
        self.focus = Focus::Tracks;
        self.selected_track = 0;
    }

    pub fn leave_search(&mut self) {
        self.search_active = false;
    }

    pub fn push_search_char(&mut self, c: char) {
        self.search_query.push(c);
        self.selected_track = 0;
    }

    pub fn pop_search_char(&mut self) {
        self.search_query.pop();
        self.selected_track = 0;
    }

    pub fn play_selected(&mut self) -> Result<()> {
        let Some(track) = self.selected_track_ref().cloned() else {
            self.message = "No sound selected".to_string();
            return Ok(());
        };

        self.audio.play(&track.path)?;
        self.playback_state = PlaybackState::Playing;
        self.now_playing = Some(track.display_name.clone());
        self.message = format!("Playing {}", track.file_name);
        self.search_active = false;
        Ok(())
    }

    pub fn toggle_pause_or_play(&mut self) -> Result<()> {
        if matches!(
            self.audio.state(),
            PlaybackState::Stopped | PlaybackState::Finished
        ) {
            self.play_selected()?;
        } else {
            self.audio.toggle_pause();
            self.playback_state = self.audio.state();
            self.message = match self.playback_state {
                PlaybackState::Paused => "Paused".to_string(),
                PlaybackState::Playing => "Resumed".to_string(),
                _ => self.message.clone(),
            };
        }

        Ok(())
    }

    pub fn stop(&mut self) {
        self.audio.stop();
        self.playback_state = PlaybackState::Stopped;
        self.message = "Stopped".to_string();
    }

    pub fn increase_volume(&mut self) {
        let next = (self.audio.volume() + 0.05).min(1.0);
        self.audio.set_volume(next);
        self.message = format!("Volume {}%", self.volume_percent());
    }

    pub fn decrease_volume(&mut self) {
        let next = (self.audio.volume() - 0.05).max(0.0);
        self.audio.set_volume(next);
        self.message = format!("Volume {}%", self.volume_percent());
    }

    pub fn volume_percent(&self) -> u8 {
        (self.audio.volume() * 100.0).round() as u8
    }

    pub fn refresh_audio_status(&mut self) {
        self.playback_state = self.audio.state();
    }
}
