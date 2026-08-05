use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use walkdir::WalkDir;

/// Audio file extensions the player understands: OGG/WAV via rodio's decoder,
/// and the tracker modules (XM/MOD/S3M/IT) rendered via `xmrs`/`xmrsplayer`.
const AUDIO_EXTS: &[&str] = &["ogg", "wav", "xm", "mod", "s3m", "it"];

fn is_audio_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(OsStr::to_str)
            .map(|e| AUDIO_EXTS.contains(&e.to_ascii_lowercase().as_str()))
            .unwrap_or(false)
}

#[derive(Debug, Clone)]
pub struct AudioLibrary {
    pub categories: Vec<Category>,
    pub total_tracks: usize,
}

#[derive(Debug, Clone)]
pub struct Category {
    pub name: String,
    pub path: PathBuf,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone)]
pub struct Track {
    pub file_name: String,
    pub display_name: String,
    pub path: PathBuf,
}

impl AudioLibrary {
    pub fn scan(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        let mut categories = Vec::new();

        for entry in root
            .read_dir()
            .with_context(|| format!("cannot read {}", root.display()))?
        {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let mut tracks = collect_tracks(&path)?;
            if tracks.is_empty() {
                continue;
            }

            tracks.sort_by(|a, b| a.file_name.cmp(&b.file_name));
            categories.push(Category {
                name: category_title(path.file_name().and_then(OsStr::to_str).unwrap_or_default()),
                path,
                tracks,
            });
        }

        // Flat root: audio files sitting DIRECTLY in `root` (no category
        // subfolders) form their own category. Lets you point `--audio-root` at a
        // flat folder — e.g. the GBA project's `sfx/` (its .xm + .wav) — not just
        // the categorised OGG pack.
        let mut root_tracks = collect_tracks(root)?;
        if !root_tracks.is_empty() {
            root_tracks.sort_by(|a, b| a.file_name.cmp(&b.file_name));
            categories.push(Category {
                name: category_title(root.file_name().and_then(OsStr::to_str).unwrap_or("Audio")),
                path: root.to_path_buf(),
                tracks: root_tracks,
            });
        }

        categories.sort_by(|a, b| a.path.cmp(&b.path));
        let total_tracks = categories
            .iter()
            .map(|category| category.tracks.len())
            .sum();

        Ok(Self {
            categories,
            total_tracks,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.total_tracks == 0
    }
}

fn collect_tracks(category_path: &Path) -> Result<Vec<Track>> {
    let mut tracks = Vec::new();

    for entry in WalkDir::new(category_path).min_depth(1).max_depth(1) {
        let entry = entry?;
        let path = entry.path();

        if !is_audio_file(path) {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or_default()
            .to_string();

        tracks.push(Track {
            display_name: track_title(&file_name),
            file_name,
            path: path.to_path_buf(),
        });
    }

    Ok(tracks)
}

pub fn category_title(raw: &str) -> String {
    let mut parts = raw.splitn(2, '_');
    let prefix = parts.next().unwrap_or_default();
    let rest = parts.next().unwrap_or_default();

    if prefix.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty() {
        format!("{} {}", prefix, title_words(rest))
    } else {
        title_words(raw)
    }
}

pub fn track_title(file_name: &str) -> String {
    // Strip any audio extension (.ogg/.wav/.xm/.mod/.s3m/.it), not just .ogg.
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or(file_name);
    title_words(stem)
}

fn title_words(raw: &str) -> String {
    raw.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| match part {
            "ui" => "UI".to_string(),
            "rpg" => "RPG".to_string(),
            "sci" => "Sci".to_string(),
            "fi" => "Fi".to_string(),
            _ => {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn formats_category_titles() {
        assert_eq!(category_title("00_core_ui"), "00 Core UI");
        assert_eq!(
            category_title("14_rpg_status_progression"),
            "14 RPG Status Progression"
        );
    }

    #[test]
    fn formats_track_titles() {
        assert_eq!(
            track_title("player_double_jump_a.ogg"),
            "Player Double Jump A"
        );
        assert_eq!(track_title("ui_select_short_c.ogg"), "UI Select Short C");
    }

    #[test]
    fn scans_categories_and_ogg_files_only() -> Result<()> {
        let temp = tempdir()?;
        let root = temp.path();
        let category = root.join("00_core_ui");
        fs::create_dir(&category)?;
        fs::write(category.join("ui_cancel_a.ogg"), [])?;
        fs::write(category.join("notes.txt"), [])?;
        fs::create_dir(root.join("empty_category"))?;

        let library = AudioLibrary::scan(root)?;

        assert_eq!(library.categories.len(), 1);
        assert_eq!(library.total_tracks, 1);
        assert_eq!(library.categories[0].name, "00 Core UI");
        assert_eq!(library.categories[0].tracks[0].file_name, "ui_cancel_a.ogg");

        Ok(())
    }

    #[test]
    fn strips_any_audio_extension() {
        assert_eq!(track_title("title.xm"), "Title");
        assert_eq!(track_title("sword_hit.wav"), "Sword Hit");
    }

    #[test]
    fn scans_flat_root_with_tracker_and_wav() -> Result<()> {
        // A flat folder (no category subdirs) — e.g. the GBA project's `sfx/`.
        let temp = tempdir()?;
        let root = temp.path();
        fs::write(root.join("title.xm"), [])?;
        fs::write(root.join("sword_hit.wav"), [])?;
        fs::write(root.join("readme.md"), [])?; // ignored (not audio)

        let library = AudioLibrary::scan(root)?;

        assert_eq!(library.categories.len(), 1);
        assert_eq!(library.total_tracks, 2);
        let files: Vec<&str> = library.categories[0]
            .tracks
            .iter()
            .map(|t| t.file_name.as_str())
            .collect();
        assert!(files.contains(&"title.xm"));
        assert!(files.contains(&"sword_hit.wav"));

        Ok(())
    }
}
