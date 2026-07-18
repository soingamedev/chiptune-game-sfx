use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use walkdir::WalkDir;

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

        if !path.is_file() || path.extension().and_then(OsStr::to_str) != Some("ogg") {
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
    let stem = file_name.strip_suffix(".ogg").unwrap_or(file_name);
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
}
