//! The skill index (WF-IMP-022).
//!
//! A skill says **how and when** a capability is used. It is not the
//! capability: the capability is a builtin in the host or a plugin under
//! `plugins/`, and the skill is the knowledge of when reaching for it is the
//! right move.
//!
//! Skills live in the agent's directory as data, one directory per skill, so
//! adding one changes the agent's behaviour without a rebuild. This module
//! reads them and resolves what they need.
//!
//! The resolution has two outcomes and only two. A skill whose required plugin
//! is present is ready. A skill whose required plugin is absent is *not* an
//! error: it is a prompt for the self-extension loop, which writes the plugin
//! and promotes it. A skill that cannot be resolved because the plugin is
//! missing would be a dead end, and a dead end teaches the operator nothing.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// One skill, as read from its directory.
#[derive(Debug, Clone)]
pub struct Skill {
    /// The name from the frontmatter, falling back to the directory name.
    pub name: String,
    /// The plugins this skill needs. A skill that requires nothing is a
    /// skill that uses only builtins.
    pub requires: Vec<String>,
    /// The body of the skill, the part the model reads.
    pub body: String,
    /// The directory the skill was read from.
    pub path: PathBuf,
}

/// How a skill resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// Everything the skill requires is present.
    Ready,
    /// A required plugin is missing. The plugin's name is what the
    /// self-extension loop would create.
    MissingPlugin { skill: String, plugin: String },
    /// No such skill. Not a missing plugin: nothing is being asked for.
    UnknownSkill { skill: String },
}

impl Resolution {
    /// The plugin that is missing, if one is.
    pub fn missing_plugin(&self) -> Option<&str> {
        match self {
            Resolution::Ready => None,
            Resolution::MissingPlugin { plugin, .. } => Some(plugin),
            Resolution::UnknownSkill { .. } => None,
        }
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Resolution::Ready)
    }
}

/// The skills one agent has.
#[derive(Debug, Default, Clone)]
pub struct SkillIndex {
    skills: Vec<Skill>,
}

impl SkillIndex {
    /// Read every skill in the directory. A directory that does not exist
    /// holds no skills; that is an empty index, not a failure.
    pub fn load(skills_dir: &Path) -> Result<Self> {
        let mut skills = Vec::new();
        if !skills_dir.is_dir() {
            return Ok(Self { skills });
        }
        let mut entries: Vec<PathBuf> = std::fs::read_dir(skills_dir)
            .map_err(|e| {
                Error::State(format!("could not read the skills directory: {e}"))
            })?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .collect();
        entries.sort();

        for directory in entries {
            if !directory.is_dir() {
                continue;
            }
            let file = directory.join("SKILL.md");
            if !file.is_file() {
                continue;
            }
            let text = std::fs::read_to_string(&file).map_err(|e| {
                Error::State(format!("could not read {}: {e}", file.display()))
            })?;
            let (frontmatter, body) = split_frontmatter(&text);
            let name = frontmatter
                .iter()
                .find(|(key, _)| key == "name")
                .map(|(_, value)| value.clone())
                .unwrap_or_else(|| {
                    directory
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_default()
                });
            let requires = frontmatter
                .iter()
                .find(|(key, _)| key == "requires")
                .map(|(_, value)| parse_list(value))
                .unwrap_or_default();
            skills.push(Skill {
                name,
                requires,
                body,
                path: directory,
            });
        }
        Ok(Self { skills })
    }

    pub fn skills(&self) -> &[Skill] {
        &self.skills
    }

    pub fn len(&self) -> usize {
        self.skills.len()
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.iter().find(|skill| skill.name == name)
    }

    /// Resolve a skill against the plugins that are installed.
    ///
    /// The first missing requirement is reported. A skill that needs three
    /// plugins and has none is one self-extension request, not three: the
    /// loop creates the plugin and the skill resolves on the next pass.
    pub fn resolve(&self, name: &str, is_installed: &dyn Fn(&str) -> bool) -> Resolution {
        let Some(skill) = self.get(name) else {
            // A skill that is not there is not a missing plugin. It is a
            // nothing, and the caller says so.
            return Resolution::UnknownSkill {
                skill: name.to_string(),
            };
        };
        for requirement in &skill.requires {
            if !is_installed(requirement) {
                return Resolution::MissingPlugin {
                    skill: skill.name.clone(),
                    plugin: requirement.clone(),
                };
            }
        }
        Resolution::Ready
    }

    /// The skills whose plugins are all present.
    pub fn ready_skills(&self, is_installed: &dyn Fn(&str) -> bool) -> Vec<&Skill> {
        self.skills
            .iter()
            .filter(|skill| self.resolve(&skill.name, is_installed).is_ready())
            .collect()
    }

    /// The skills that are waiting for a plugin, and the plugin each waits for.
    pub fn blocked_skills(&self, is_installed: &dyn Fn(&str) -> bool) -> Vec<(String, String)> {
        self.skills
            .iter()
            .filter_map(|skill| match self.resolve(&skill.name, is_installed) {
                Resolution::Ready | Resolution::UnknownSkill { .. } => None,
                Resolution::MissingPlugin { plugin, .. } => {
                    Some((skill.name.clone(), plugin))
                }
            })
            .collect()
    }
}

/// Split a document into its frontmatter and its body. A document with no
/// frontmatter is all body.
fn split_frontmatter(text: &str) -> (Vec<(String, String)>, String) {
    let trimmed = text.trim_start_matches('\u{feff}');
    let Some(rest) = trimmed.strip_prefix("---\n") else {
        return (Vec::new(), text.to_string());
    };
    let Some(end) = rest.find("\n---") else {
        return (Vec::new(), text.to_string());
    };
    let frontmatter = &rest[..end];
    let body = rest[end + 4..].trim_start_matches('\n').to_string();
    (parse_frontmatter(frontmatter), body)
}

/// Read `key: value` pairs. A value that continues on the next indented lines
/// is a list, and is kept as the lines it spans.
fn parse_frontmatter(text: &str) -> Vec<(String, String)> {
    let mut fields: Vec<(String, String)> = Vec::new();
    let mut current: Option<(String, String)> = None;
    for line in text.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_string();
            if key.is_empty() || key.contains(' ') {
                // Not a field: a continuation of the value above.
                if let Some((_, value)) = current.as_mut() {
                    value.push('\n');
                    value.push_str(line.trim());
                }
                continue;
            }
            if let Some(previous) = current.take() {
                fields.push(previous);
            }
            current = Some((key, value.trim().to_string()));
        } else if let Some((_, value)) = current.as_mut() {
            value.push('\n');
            value.push_str(line.trim());
        }
    }
    if let Some(previous) = current.take() {
        fields.push(previous);
    }
    fields
}

/// A list of names, written either inline (`[a, b]`) or one per line under a
/// key.
fn parse_list(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    // The block form is detected before trimming: a leading newline is what
    // distinguishes it, and trimming throws that away.
    let is_block = value.contains('\n') || trimmed.starts_with('-');
    if is_block {
        // A block list: one entry per line, each optionally prefixed with a
        // dash. A line may still hold several comma-separated names.
        return value
            .lines()
            .flat_map(|line| {
                line.trim()
                    .trim_start_matches('-')
                    .split(',')
                    .map(|entry| entry.trim().trim_matches(['"', '\'']).to_string())
                    .collect::<Vec<String>>()
            })
            .filter(|entry| !entry.is_empty())
            .collect();
    }
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(trimmed);
    inner
        .split(',')
        .map(|entry| entry.trim().trim_matches(['"', '\'']).to_string())
        .filter(|entry| !entry.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_skill(root: &Path, directory: &str, contents: &str) {
        let path = root.join(directory);
        std::fs::create_dir_all(&path).expect("the skill directory");
        std::fs::write(path.join("SKILL.md"), contents).expect("the skill file");
    }

    fn scratch(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "agent-skills-test-{name}-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("the skills directory");
        root
    }

    #[test]
    fn a_skill_is_read_from_its_directory() {
        let root = scratch("read");
        write_skill(
            &root,
            "turn-off-the-lights",
            "---\nname: turn-off-the-lights\nrequires: [home.lights]\n---\n\nUse when the user asks for a light to be switched off.",
        );
        let index = SkillIndex::load(&root).expect("the index loads");
        assert_eq!(index.len(), 1);
        let skill = index.get("turn-off-the-lights").expect("the skill");
        assert_eq!(skill.requires, vec!["home.lights".to_string()]);
        assert!(skill.body.contains("switched off"), "{}", skill.body);
    }

    #[test]
    fn a_skill_that_requires_nothing_is_ready() {
        let root = scratch("builtin");
        write_skill(
            &root,
            "greet",
            "---\nname: greet\n---\n\nUse when the user says hello.",
        );
        let index = SkillIndex::load(&root).expect("the index loads");
        let resolution = index.resolve("greet", &|_| false);
        assert!(resolution.is_ready(), "{resolution:?}");
    }

    /// A skill whose plugin is absent is a prompt for the self-extension loop,
    /// not a dead end.
    #[test]
    fn a_missing_plugin_produces_a_load_or_create_path() {
        let root = scratch("missing");
        write_skill(
            &root,
            "turn-off-the-lights",
            "---\nname: turn-off-the-lights\nrequires: [home.lights]\n---\n\nUse when a light is switched off.",
        );
        let index = SkillIndex::load(&root).expect("the index loads");

        let resolution = index.resolve("turn-off-the-lights", &|_| false);
        assert_eq!(
            resolution,
            Resolution::MissingPlugin {
                skill: "turn-off-the-lights".to_string(),
                plugin: "home.lights".to_string(),
            }
        );
        assert_eq!(resolution.missing_plugin(), Some("home.lights"));
        assert!(!resolution.is_ready());

        // The same skill resolves once the plugin exists.
        let installed = |name: &str| name == "home.lights";
        assert!(index.resolve("turn-off-the-lights", &installed).is_ready());
        assert_eq!(index.ready_skills(&installed).len(), 1);
        assert!(index.blocked_skills(&installed).is_empty());
    }

    #[test]
    fn a_blocked_skill_names_the_plugin_it_waits_for() {
        let root = scratch("blocked");
        write_skill(
            &root,
            "weather",
            "---\nname: weather\nrequires:\n  - weather.now\n---\n\nUse when the user asks about the weather.",
        );
        let index = SkillIndex::load(&root).expect("the index loads");
        assert_eq!(
            index.get("weather").expect("the skill").requires,
            vec!["weather.now".to_string()]
        );
        let blocked = index.blocked_skills(&|_| false);
        assert_eq!(
            blocked,
            vec![("weather".to_string(), "weather.now".to_string())]
        );
    }

    /// Adding a skill file changes the agent's behaviour with no rebuild: the
    /// index is read from the directory, so a new directory is a new skill.
    #[test]
    fn a_new_directory_is_a_new_skill() {
        let root = scratch("added");
        let before = SkillIndex::load(&root).expect("the index loads");
        assert!(before.is_empty());

        write_skill(
            &root,
            "weather",
            "---\nname: weather\nrequires: [weather.now]\n---\n\nUse when the user asks about the weather.",
        );
        let after = SkillIndex::load(&root).expect("the index loads");
        assert_eq!(after.len(), 1);
        assert!(after.get("weather").is_some());
    }

    #[test]
    fn a_directory_without_a_skill_file_is_not_a_skill() {
        let root = scratch("empty");
        std::fs::create_dir_all(root.join("not-a-skill")).expect("the directory");
        let index = SkillIndex::load(&root).expect("the index loads");
        assert!(index.is_empty());
    }

    #[test]
    fn a_missing_skills_directory_holds_no_skills() {
        let index = SkillIndex::load(Path::new("/nonexistent/skills")).expect("the index loads");
        assert!(index.is_empty());
    }
}
