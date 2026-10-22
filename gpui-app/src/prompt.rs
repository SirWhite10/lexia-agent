//! Prompt assembly (WF-IMP-005, ported by WF-IMP-022).
//!
//! The slot order is the cache contract from WF-DEC-002:
//! `[personality → all skills → retrieved memory → resolved tools]`. Everything
//! above the memory block is byte-stable across turns so provider prompt
//! caching hits; only the suffix moves as memory and capabilities change. The
//! prefix digest makes that property testable.
//!
//! This module is deliberately pure: it opens no database and reads no files,
//! so the runtime supplies the personality block and the retrieval layer
//! supplies memory. Keeping it side-effect free also keeps the cache contract
//! provable without standing up state.
//!
//! The TypeScript original keeps one module-level cursor, because the
//! stability contract is a property of *consecutive* assemblies from the single
//! runtime that owns them. Here the cursor belongs to an [`PromptAssembler`],
//! and the host owns one. The behaviour is the same; the ownership is
//! explicit rather than ambient.

use sha2::{Digest, Sha256};

/// The agent's identity block, rewritten only at reflection boundaries.
#[derive(Debug, Default, Clone)]
pub struct PromptSlots {
    /// Agent's personality block; rewritten only at reflection boundaries.
    pub personality: Option<String>,
    /// The complete skill set, always injected in full, never selected per turn.
    pub skills: Vec<String>,
    /// Retrieved memory entries, already budget-capped by the retrieval layer.
    pub memory: Vec<String>,
    /// Capabilities the router resolved for this request.
    pub tools: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AssembledPrompt {
    /// The full system prompt in slot order.
    pub system: String,
    /// Digest of everything above the memory block; equal across turns until
    /// personality or skills change.
    pub prefix_digest: String,
    /// Digest of the volatile suffix; changes whenever memory or tools change.
    pub suffix_digest: String,
    /// True when personality and skills are byte-identical to the previous
    /// assembly.
    pub prefix_stable: bool,
}

const HEADING_PERSONALITY: &str = "# Personality";
const HEADING_SKILLS: &str = "# Skills";
const HEADING_MEMORY: &str = "# Memory";
const HEADING_TOOLS: &str = "# Tools";

fn section(heading: &str, body: &str) -> String {
    format!("{heading}\n\n{}\n", body.trim())
}

fn digest(parts: &[String]) -> String {
    // The original joins the parts with a space and hashes the result once, so
    // the digest is the same value on both sides. Hashing each part separately
    // would produce a different digest, and a digest that differs between the
    // two implementations cannot be compared.
    let joined = parts.join(" ");
    let out = Sha256::digest(joined.as_bytes());
    out.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Assembles turns and remembers the previous prefix.
#[derive(Debug, Default)]
pub struct PromptAssembler {
    previous_prefix_digest: Option<String>,
}

impl PromptAssembler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Assemble one turn's system prompt.
    pub fn assemble(&mut self, slots: &PromptSlots) -> AssembledPrompt {
        let mut prefix_parts: Vec<String> = Vec::new();
        if slots
            .personality
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        {
            prefix_parts.push(section(HEADING_PERSONALITY, slots.personality.as_deref().unwrap()));
        }
        if !slots.skills.is_empty() {
            prefix_parts.push(section(HEADING_SKILLS, &slots.skills.join("\n\n")));
        }

        let mut suffix_parts: Vec<String> = Vec::new();
        if !slots.memory.is_empty() {
            suffix_parts.push(section(HEADING_MEMORY, &slots.memory.join("\n\n")));
        }
        if !slots.tools.is_empty() {
            suffix_parts.push(section(HEADING_TOOLS, &slots.tools.join("\n\n")));
        }

        let prefix_digest = digest(&prefix_parts);
        let suffix_digest = digest(&suffix_parts);
        let prefix_stable = self
            .previous_prefix_digest
            .as_ref()
            .is_none_or(|previous| previous == &prefix_digest);
        self.previous_prefix_digest = Some(prefix_digest.clone());

        AssembledPrompt {
            system: prefix_parts
                .iter()
                .chain(suffix_parts.iter())
                .cloned()
                .collect::<Vec<String>>()
                .join("\n"),
            prefix_digest,
            suffix_digest,
            prefix_stable,
        }
    }

    /// Test seam: resets the consecutive-assembly cursor.
    pub fn reset(&mut self) {
        self.previous_prefix_digest = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SKILLS: [&str; 2] = [
        "# Skill: lights\nOperate the smart home.",
        "# Skill: calendar\nRead and write events.",
    ];
    const PERSONA: &str = "# Personality\n\n- [2026-01-01T00:00:00.000Z] answers concisely";

    fn slots(personality: Option<&str>, memory: &[&str], tools: &[&str]) -> PromptSlots {
        PromptSlots {
            personality: personality.map(str::to_string),
            skills: SKILLS.iter().map(|s| s.to_string()).collect(),
            memory: memory.iter().map(|s| s.to_string()).collect(),
            tools: tools.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Ported from `tests/prompt-assembly.test.ts`. The same four cases, the
    /// same claims.
    #[test]
    fn two_turns_differing_only_in_memory_share_the_prefix_digest() {
        let mut assembler = PromptAssembler::new();
        let first = assembler.assemble(&slots(Some(PERSONA), &["fact A"], &["lights.off"]));
        assert!(first.prefix_stable);

        let second = assembler.assemble(&slots(Some(PERSONA), &["fact B", "fact C"], &["lights.off"]));
        assert!(second.prefix_stable);
        assert_eq!(second.prefix_digest, first.prefix_digest);
        assert_ne!(second.suffix_digest, first.suffix_digest);
    }

    #[test]
    fn a_personality_rewrite_changes_the_prefix_once_at_the_boundary() {
        let mut assembler = PromptAssembler::new();
        assembler.assemble(&slots(Some(PERSONA), &[], &[]));
        let grown = format!("{PERSONA}\n- [2026-01-02T00:00:00.000Z] asks before acting");
        let rewritten = assembler.assemble(&slots(Some(&grown), &[], &[]));
        assert!(!rewritten.prefix_stable);
        let after = assembler.assemble(&slots(Some(&grown), &["fact"], &[]));
        assert!(after.prefix_stable);
        assert_eq!(after.prefix_digest, rewritten.prefix_digest);
    }

    #[test]
    fn slot_order_is_personality_skills_memory_tools() {
        let mut assembler = PromptAssembler::new();
        let prompt = assembler.assemble(&slots(
            Some(PERSONA),
            &["remembered fact"],
            &["resolved capability"],
        ));
        let order: Vec<Option<usize>> = [HEADING_PERSONALITY, HEADING_SKILLS, HEADING_MEMORY, HEADING_TOOLS]
            .iter()
            .map(|heading| prompt.system.find(heading))
            .collect();
        assert!(order.iter().all(|index| index.is_some()), "{order:?}");
        let positions: Vec<usize> = order.into_iter().map(|index| index.unwrap()).collect();
        let mut sorted = positions.clone();
        sorted.sort_unstable();
        assert_eq!(positions, sorted, "the headings are out of order: {positions:?}");
    }

    #[test]
    fn empty_slots_are_omitted_not_rendered_as_empty_headings() {
        let mut assembler = PromptAssembler::new();
        let empty = assembler.assemble(&PromptSlots::default());
        assert_eq!(empty.system, "");
        let with_memory = assembler.assemble(&PromptSlots {
            memory: vec!["fact".to_string()],
            ..Default::default()
        });
        assert!(with_memory.system.contains(HEADING_MEMORY));
        assert!(!with_memory.system.contains(HEADING_TOOLS));
    }

    /// Byte-for-byte parity with the implementation this was ported from.
    ///
    /// The digests below were produced by the original
    /// `src/lib/server/prompt.ts` for this exact input. A digest is only
    /// comparable between two implementations if they compute it the same
    /// way, so the values are pinned here rather than merely compared to each
    /// other.
    #[test]
    fn the_digest_matches_the_implementation_this_was_ported_from() {
        let mut assembler = PromptAssembler::new();
        let prompt = assembler.assemble(&slots(Some(PERSONA), &["fact A"], &["lights.off"]));
        assert_eq!(
            prompt.prefix_digest,
            "c063b0697fc058d51cd61a5aa705ded5291758ed318e900ba1f75748994d7f57"
        );
        assert_eq!(
            prompt.suffix_digest,
            "a2b36f138cc5d1874187650d8b0c7a0c3a22a9687e29732797ece8f19ec93a39"
        );
        assert_eq!(prompt.system.len(), 209, "the prompt is the same bytes");
    }
}
