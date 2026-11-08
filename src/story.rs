//! Stories: what the stage shows, step by step, each step its panels, what
//! fills them and the words that go with it. Every story is kept on disk as
//! it is told, so that it can be told again at once, and found again by
//! what it is about.
//!
//! The library is a folder of JSON files, one per story, under the user's
//! data directory: `~/.local/share/cellstage`, or `$CELLSTAGE_HOME` when it
//! is set.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A tool call as the agent made it, kept to be made again.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Call {
    pub tool: String,
    pub args: Value,
}

/// What one panel of a step shows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelSpec {
    pub id: String,
    pub title: String,
    /// The `show_*` call that filled it.
    #[serde(default)]
    pub show: Option<Call>,
    /// The notes written and the turns of the camera made after, in order.
    #[serde(default)]
    pub after: Vec<Call>,
}

/// One step of a story: a layout, what fills it, and what is said.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub title: String,
    #[serde(default)]
    pub narration: String,
    #[serde(default)]
    pub arrangement: String,
    #[serde(default)]
    pub panels: Vec<PanelSpec>,
}

/// A story, as kept in the library.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Story {
    /// Its file name, made from its title.
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Seconds since 1970, when it was begun and last changed.
    pub created: u64,
    pub updated: u64,
    pub steps: Vec<Step>,
}

impl Story {
    /// A new story with one empty step, under an id no other story has.
    pub fn new(title: &str, summary: &str, tags: Vec<String>) -> Self {
        let base = slug(title);
        let base = if base.is_empty() {
            "story".to_owned()
        } else {
            base
        };
        let mut id = base.clone();
        let mut n = 2;
        while home().join("stories").join(format!("{id}.json")).exists() {
            id = format!("{base}-{n}");
            n += 1;
        }
        let now = now();
        Self {
            id,
            title: title.to_owned(),
            summary: summary.to_owned(),
            tags,
            created: now,
            updated: now,
            steps: vec![Step {
                title: title.to_owned(),
                ..Step::default()
            }],
        }
    }

    /// Write it to the library.
    pub fn save(&mut self) -> anyhow::Result<()> {
        self.updated = now();
        let dir = home().join("stories");
        std::fs::create_dir_all(&dir)?;
        std::fs::write(
            dir.join(format!("{}.json", self.id)),
            serde_json::to_string_pretty(self)?,
        )?;
        Ok(())
    }

    /// Read one from the library.
    pub fn load(id: &str) -> anyhow::Result<Self> {
        let file = home().join("stories").join(format!("{}.json", slug(id)));
        let text = std::fs::read_to_string(&file)
            .map_err(|e| anyhow::anyhow!("no story {id} in the library ({e})"))?;
        Ok(serde_json::from_str(&text)?)
    }

    /// Every story in the library, the last changed first.
    pub fn all() -> Vec<Self> {
        let mut stories: Vec<Self> = read_dir("stories");
        stories.sort_by_key(|s| std::cmp::Reverse(s.updated));
        stories
    }

    /// The story in a few lines: what it is, and its steps.
    pub fn outline(&self) -> String {
        let steps: Vec<String> = self
            .steps
            .iter()
            .enumerate()
            .map(|(i, s)| format!("  {}. {}", i + 1, s.title))
            .collect();
        format!(
            "story {}: \"{}\", {} steps, changed {}{}{}\n{}",
            self.id,
            self.title,
            self.steps.len(),
            date(self.updated),
            if self.summary.is_empty() {
                String::new()
            } else {
                format!(". {}", self.summary)
            },
            if self.tags.is_empty() {
                String::new()
            } else {
                format!(" (tags: {})", self.tags.join(", "))
            },
            steps.join("\n")
        )
    }

    /// The words a search looks through, each with its weight.
    fn words(&self) -> Vec<(String, u32)> {
        let mut out = vec![
            (fold(&self.title), 4),
            (fold(&self.tags.join(" ")), 4),
            (fold(&self.summary), 2),
        ];
        for step in &self.steps {
            out.push((fold(&step.title), 2));
            out.push((fold(&step.narration), 1));
            for p in &step.panels {
                out.push((fold(&p.title), 1));
            }
        }
        out
    }
}

/// Stories that speak of what `query` asks, the best first. An empty query
/// lists the latest stories.
pub fn search(query: &str) -> Vec<Story> {
    let terms: Vec<String> = fold(query)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 1 && !STOP.contains(w))
        .map(str::to_owned)
        .collect();
    if terms.is_empty() {
        return Story::all().into_iter().take(15).collect();
    }
    let score = |words: &[(String, u32)]| -> u32 {
        terms
            .iter()
            .map(|t| {
                words
                    .iter()
                    .filter(|(w, _)| matches(w, t))
                    .map(|(_, weight)| *weight)
                    .max()
                    .unwrap_or(0)
            })
            .sum()
    };
    let mut found: Vec<(u32, Story)> = Vec::new();
    for s in Story::all() {
        let n = score(&s.words());
        if n > 0 {
            found.push((n, s));
        }
    }
    found.sort_by_key(|(n, _)| std::cmp::Reverse(*n));
    found.into_iter().take(15).map(|(_, f)| f).collect()
}

/// Whether a word of `text` begins with `term`, so that "melanocyte"
/// finds "melanocytes" and "melan" finds both.
fn matches(text: &str, term: &str) -> bool {
    text.split(|c: char| !c.is_alphanumeric())
        .any(|w| w.starts_with(term) || (term.len() > 4 && w.starts_with(&term[..term.len() - 1])))
}

/// Words a search ignores, in French and English.
const STOP: [&str; 30] = [
    "le", "la", "les", "de", "des", "du", "un", "une", "et", "en", "au", "aux", "dans", "sur",
    "pour", "par", "qui", "que", "est", "the", "of", "and", "in", "on", "to", "for", "is", "how",
    "what", "comment",
];

/// Where the library lives.
pub fn home() -> PathBuf {
    #[cfg(test)]
    if let Some(dir) = tests::HOME.with(|h| h.borrow().clone()) {
        return dir;
    }
    if let Some(dir) = std::env::var_os("CELLSTAGE_HOME") {
        return PathBuf::from(dir);
    }
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(std::env::temp_dir)
        .join("cellstage")
}

fn read_dir<T: serde::de::DeserializeOwned>(sub: &str) -> Vec<T> {
    let Ok(entries) = std::fs::read_dir(home().join(sub)) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| serde_json::from_str(&std::fs::read_to_string(e.path()).ok()?).ok())
        .collect()
}

/// Lower case, without accents, so that "Crête" and "crete" meet.
pub fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.to_lowercase().chars() {
        match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => out.push('a'),
            'é' | 'è' | 'ê' | 'ë' => out.push('e'),
            'î' | 'ï' | 'í' | 'ì' => out.push('i'),
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => out.push('o'),
            'ù' | 'û' | 'ü' | 'ú' => out.push('u'),
            'ç' => out.push('c'),
            'ñ' => out.push('n'),
            'œ' => out.push_str("oe"),
            'æ' => out.push_str("ae"),
            'ÿ' => out.push('y'),
            c => out.push(c),
        }
    }
    out
}

/// A name for a file: folded, letters and digits joined by dashes.
pub fn slug(text: &str) -> String {
    let folded = fold(text);
    let words: Vec<&str> = folded
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let mut out = words.join("-");
    out.truncate(60);
    out.trim_end_matches('-').to_owned()
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// A day as year-month-day, from seconds since 1970 (Hinnant's civil
/// calendar).
pub fn date(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64 + 719_468;
    let era = days.div_euclid(146_097);
    let doe = days - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_fold_accents_and_join_words() {
        assert_eq!(slug("La crête neurale : départ"), "la-crete-neurale-depart");
        assert_eq!(slug("  RAS, G12  "), "ras-g12");
    }

    #[test]
    fn a_search_finds_words_by_their_start() {
        assert!(matches("les melanocytes migrent", "melanocyte"));
        assert!(matches("crete neurale", "crete"));
        assert!(!matches("leucemie", "crete"));
    }

    #[test]
    fn dates_read_as_the_calendar_does() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(1_794_048_000), "2026-11-07");
    }

    #[test]
    fn a_story_is_kept_and_found_again() {
        let dir = std::env::temp_dir().join(format!("cellstage-test-{}", std::process::id()));
        HOME.with(|h| *h.borrow_mut() = Some(dir.clone()));
        let mut story = Story::new(
            "La crête neurale",
            "d'où viennent les mélanocytes",
            vec!["embryon".into()],
        );
        story.steps[0].narration = "Les cellules quittent le tube neural.".into();
        story.save().expect("saved");
        let found = search("melanocytes crete");
        assert!(found.first().is_some_and(|s| s.id == "la-crete-neurale"));
        assert_eq!(
            Story::load("la-crete-neurale").expect("loaded").steps.len(),
            1
        );
        let again = Story::new("La crête neurale", "", Vec::new());
        assert_eq!(again.id, "la-crete-neurale-2");
        let _ = std::fs::remove_dir_all(&dir);
    }

    thread_local! {
        /// The library a test works in, instead of the user's.
        pub static HOME: std::cell::RefCell<Option<PathBuf>> =
            const { std::cell::RefCell::new(None) };
    }
}
