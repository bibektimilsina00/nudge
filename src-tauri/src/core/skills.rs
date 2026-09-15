//! Things somebody taught Nudge to do, kept in a folder.
//!
//! A skill is a directory with a `SKILL.md` in it:
//!
//! ```text
//! ~/.config/nudge/skills/
//!   weekly-report/
//!     SKILL.md
//!   tidy-downloads/
//!     SKILL.md
//! ```
//!
//! And `SKILL.md` is a name, a description, and then the instructions:
//!
//! ```text
//! ---
//! name: Weekly report
//! description: Collects the week's commits and writes the Friday summary.
//! ---
//!
//! 1. Run `git log --since="last monday"` in each project folder.
//! 2. Group by project, newest first.
//! ...
//! ```
//!
//! **Deliberately the shape the rest of the world already uses.** A folder with a
//! `SKILL.md` carrying name and description in frontmatter is what Claude Code and
//! the agent-skills ecosystem settled on, which means a skill somebody already
//! wrote works here without being rewritten, and one written here is not trapped.
//! Inventing a format would buy nothing and cost every skill that already exists.
//!
//! ## Only the description is ever loaded
//!
//! The prompt carries each skill's **name and one line**, never the instructions.
//! Twenty skills is then a couple of hundred tokens on every turn instead of
//! twenty thousand -- and the instructions arrive only when one is actually
//! opened, which is the same progressive disclosure the tool list uses and for the
//! same reason: Nudge pays for its context on every hotkey press.
use std::path::PathBuf;

/// Longest a skill's instructions may be.
///
/// Generous -- this is read once, when the skill is chosen, not on every turn.
/// Bounded anyway, because a file that turns out to be a video does not belong in
/// a prompt.
const LONGEST: usize = 24_000;

/// One skill, as its folder describes it.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Skill {
    /// What it is called, from the frontmatter, falling back to the folder name.
    pub name: String,
    /// The one line that decides whether it is the right skill. This is the only
    /// part that reaches the model unasked.
    pub about: String,
    /// The folder, so the interface can reveal it.
    pub folder: PathBuf,
}

/// Where skills live. Beside the config, which is where somebody would look.
pub fn folder() -> Option<PathBuf> {
    dirs::home_dir().map(|d| d.join(".config/nudge/skills"))
}

/// Everything installed, in a stable order.
///
/// Read from disk each time rather than cached. Somebody who has just dropped a
/// folder in expects to see it, and a list nobody can refresh is the kind of
/// thing people restart an application to fix.
pub fn installed() -> Vec<Skill> {
    let Some(root) = folder() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut found: Vec<Skill> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter_map(|e| read(&e.path()))
        .collect();
    // By name, so the list does not reshuffle itself between openings for reasons
    // nobody can see.
    found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    found
}

/// Read one folder, if it holds a skill.
fn read(dir: &std::path::Path) -> Option<Skill> {
    let text = std::fs::read_to_string(dir.join("SKILL.md")).ok()?;
    let (front, _) = split(&text);
    let fallback = dir.file_name()?.to_string_lossy().replace(['-', '_'], " ");
    Some(Skill {
        name: field(&front, "name").unwrap_or(fallback),
        about: field(&front, "description").unwrap_or_default(),
        folder: dir.to_path_buf(),
    })
}

/// The instructions, fetched when a skill is actually chosen.
pub fn open(name: &str) -> Option<String> {
    let skill = installed()
        .into_iter()
        .find(|s| s.name.eq_ignore_ascii_case(name.trim()))?;
    let text = std::fs::read_to_string(skill.folder.join("SKILL.md")).ok()?;
    let (_, body) = split(&text);
    Some(body.chars().take(LONGEST).collect())
}

/// Separate the frontmatter from the instructions.
///
/// Hand-rolled rather than a YAML dependency: the whole grammar in use here is
/// `key: value`, and a parser that accepts anchors and multi-line scalars is a
/// larger surface than the thing it is parsing.
fn split(text: &str) -> (String, String) {
    let text = text.trim_start_matches('\u{feff}');
    let Some(rest) = text.strip_prefix("---") else {
        return (String::new(), text.to_string());
    };
    let Some(end) = rest.find("\n---") else {
        // An opening fence with no closing one is a file being edited, not a file
        // with no body. Treat the whole thing as instructions.
        return (String::new(), text.to_string());
    };
    let front = rest[..end].to_string();
    let body = rest[end + 4..].trim_start_matches(['\n', '\r']).to_string();
    (front, body)
}

fn field(front: &str, key: &str) -> Option<String> {
    front.lines().find_map(|line| {
        let (k, v) = line.split_once(':')?;
        if !k.trim().eq_ignore_ascii_case(key) {
            return None;
        }
        let v = v.trim().trim_matches(['"', '\'']).trim();
        (!v.is_empty()).then(|| v.to_string())
    })
}

/// What the prompt says about them: names and one line each, never the
/// instructions.
pub fn prompt() -> String {
    let skills = installed();
    if skills.is_empty() {
        return String::new();
    }
    format!(
        "## Skills\n\n\
         Things this person has taught you, or installed. If one of them is what \
         is being asked for, answer `skill` with its exact name and you will be \
         given its instructions to follow. Do not guess at what a skill does from \
         its name.\n{}\n\n",
        skills
            .iter()
            .map(|s| match s.about.is_empty() {
                true => format!("- **{}**", s.name),
                false => format!("- **{}** -- {}", s.name, s.about),
            })
            .collect::<Vec<_>>()
            .join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &std::path::Path, name: &str, text: &str) {
        let d = dir.join(name);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("SKILL.md"), text).unwrap();
    }

    /// Against whatever is really installed, rather than a fixture.
    #[test]
    #[ignore = "reads the real skills folder"]
    fn show_me() {
        for s in installed() {
            println!("  {} -- {}", s.name, s.about);
        }
        println!("\n--- prompt ---\n{}", prompt());
    }

    #[test]
    fn frontmatter_is_taken_apart_from_the_instructions() {
        let (front, body) = split("---\nname: Tidy\ndescription: Sorts things.\n---\n\nStep one.\n");
        assert_eq!(field(&front, "name").as_deref(), Some("Tidy"));
        assert_eq!(field(&front, "description").as_deref(), Some("Sorts things."));
        assert_eq!(body, "Step one.\n");
    }

    /// A file being edited has an opening fence and no closing one. That is not a
    /// file with no instructions in it.
    #[test]
    fn an_unclosed_fence_is_all_instructions() {
        let (front, body) = split("---\nname: Half written\n");
        assert!(front.is_empty());
        assert!(body.contains("Half written"));
    }

    #[test]
    fn a_plain_markdown_file_is_all_instructions() {
        let (front, body) = split("# Just notes\n\nDo the thing.");
        assert!(front.is_empty());
        assert!(body.starts_with("# Just notes"));
    }

    #[test]
    fn quotes_around_a_value_are_not_part_of_it() {
        let (front, _) = split("---\nname: \"Weekly report\"\n---\nx");
        assert_eq!(field(&front, "name").as_deref(), Some("Weekly report"));
    }

    /// A folder with no frontmatter is still a skill; it is just named after
    /// itself. Refusing it would mean the simplest possible skill does not work.
    #[test]
    fn a_folder_without_a_name_is_named_after_itself() {
        let dir = std::env::temp_dir().join("nudge-skills-test-name");
        let _ = std::fs::remove_dir_all(&dir);
        write(&dir, "tidy-downloads", "Just do it.");
        let s = read(&dir.join("tidy-downloads")).unwrap();
        assert_eq!(s.name, "tidy downloads");
        assert_eq!(s.about, "");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_with_no_skill_file_is_not_a_skill() {
        let dir = std::env::temp_dir().join("nudge-skills-test-empty");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("not-a-skill")).unwrap();
        assert!(read(&dir.join("not-a-skill")).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
