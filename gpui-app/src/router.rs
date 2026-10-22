//! The turn router (WF-IMP-023).
//!
//! Reads a request and decides the run graph behind it. Today it recognises
//! exactly one intent — delegate this to a sub-agent — from the wording, and
//! hands back the role card and the task to run.
//!
//! That is a heuristic on purpose, and it is not the final shape: the router
//! that replaces it will read the thought tree instead of a list of patterns.
//! What matters here is that the decision is made in one place and produces a
//! real run graph, so swapping the decision out later changes nothing
//! downstream. The seam is [`TurnPlan`]: whatever reads it sees one typed
//! decision, never a pattern.
//!
//! **On the matching.** The original is a set of regular expressions, several
//! of them with lazy quantifiers the `regex` crate cannot express. This
//! matches on word sequences instead, which is the same shape without the
//! engine. One behaviour differs, and it is the only one: whitespace between
//! the words of a phrase is not significant here, where the original's single
//! space literals made it significant. "hand  it  off" is a hand-off here and
//! was not one there.

use crate::registry::Capability;

/// A sub-agent the turn spawned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubAgentPlan {
    /// Role-card name, also the sidebar label.
    pub name: String,
    /// One line of scope, shown under the name in the sub-agent manager.
    pub focus: String,
    /// Version 1 of the sub-agent's prompt: who it is and what it answers.
    pub system_prompt: String,
    /// The single job this sub-agent was spawned for.
    pub task: String,
}

/// The decision for one turn.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TurnPlan {
    /// The sub-agent to run, when the turn delegates.
    pub subagent: Option<SubAgentPlan>,
}

impl TurnPlan {
    /// The capability the plan needs, when it has one.
    pub fn capability(&self) -> Option<Capability> {
        self.subagent.as_ref().map(|_| Capability::SubagentSpawn)
    }
}

/// A word in the body, with where it was.
#[derive(Debug, Clone)]
struct Word {
    /// The word, lowercased.
    text: String,
    /// Where it starts in the body.
    start: usize,
    /// Where it ends in the body.
    end: usize,
}

/// The body's words. A hyphen or a space separates words, so "sub-agent" and
/// "sub agent" are the same two words.
fn words(body: &str) -> Vec<Word> {
    let mut found = Vec::new();
    let mut start: Option<usize> = None;
    for (index, character) in body.char_indices() {
        if character.is_alphanumeric() {
            if start.is_none() {
                start = Some(index);
            }
        } else if let Some(begin) = start.take() {
            found.push(Word {
                text: body[begin..index].to_lowercase(),
                start: begin,
                end: index,
            });
        }
    }
    if let Some(begin) = start {
        found.push(Word {
            text: body[begin..].to_lowercase(),
            start: begin,
            end: body.len(),
        });
    }
    found
}

/// The word sequences that mean "this is not my job". "spin up" alone is not
/// one of them — people say it about kettles.
fn delegation_phrases() -> Vec<Vec<&'static str>> {
    let mut phrases: Vec<Vec<&'static str>> = Vec::new();

    // A sub-agent, however it is spelled.
    for tail in ["agent", "agents"] {
        phrases.push(vec!["sub", tail]);
    }
    phrases.push(vec!["subagent"]);
    phrases.push(vec!["subagents"]);

    // An explicit hand-off.
    phrases.push(vec!["delegate"]);
    for it in ["this", "it", "that"] {
        for direction in ["off", "over"] {
            phrases.push(vec!["hand", it, direction]);
        }
    }

    // Someone else is asked to do the work.
    for verb in ["have", "get"] {
        for who in ["someone", "another", "one", "it", "them"] {
            let mut base = vec![verb, who];
            // "another one" is two words, and the alternatives are exclusive:
            // "another" alone is not a person.
            if who == "another" {
                base = vec![verb, "another", "one"];
            }
            for work in ["do", "look", "research", "check", "find", "dig"] {
                let mut sequence = base.clone();
                sequence.push(work);
                phrases.push(sequence.clone());
                let mut with_to = base.clone();
                with_to.push("to");
                with_to.push(work);
                phrases.push(with_to);
            }
        }
    }

    // Asking someone, in so many words.
    for who in [
        vec!["someone"],
        vec!["another", "agent"],
        vec!["a", "sub", "agent"],
        vec!["a", "subagent"],
    ] {
        let mut sequence = vec!["ask"];
        sequence.extend(who);
        sequence.push("to");
        phrases.push(sequence);
    }

    // Spinning one up.
    for article in ["", "a", "an"] {
        for role in ["agent", "helper", "researcher"] {
            let mut sequence = vec!["spin", "up"];
            if !article.is_empty() {
                sequence.push(article);
            }
            sequence.push(role);
            phrases.push(sequence);
        }
    }

    phrases
}

/// The roles the request itself named; used when there is no topic to name the
/// sub-agent after.
const NAMED_ROLES: [&str; 6] = [
    "researcher",
    "analyst",
    "writer",
    "reviewer",
    "engineer",
    "specialist",
];

/// Where a delegation phrase starts, and how long it is.
fn find_delegation(words: &[Word], phrases: &[Vec<&'static str>]) -> Option<(usize, usize)> {
    for phrase in phrases {
        if phrase.len() > words.len() {
            continue;
        }
        for start in 0..=(words.len() - phrase.len()) {
            if words[start..start + phrase.len()]
                .iter()
                .zip(phrase)
                .all(|(word, expected)| word.text == *expected)
            {
                return Some((start, phrase.len()));
            }
        }
    }
    None
}

/// The end of the hand-off clause: the first break after the phrase.
///
/// The clause runs to the first break — a comma, or the conjunction that
/// starts the actual work. It is lazy on both sides: greedy would run to the
/// end of the sentence and treat the whole request as the hand-off clause.
fn clause_end(body: &str, words: &[Word], phrase_end: usize) -> usize {
    let after = words[phrase_end - 1].end;

    // A punctuation break.
    let punctuation = body[after..]
        .find([',', '.', ';', '\n'])
        .map(|offset| after + offset);

    // A conjunction that starts the actual work. "and then" is one break, not
    // two, so the clause ends after the "then".
    let mut conjunction: Option<usize> = None;
    for index in phrase_end..words.len() {
        if words[index].text == "and" {
            let ends_after = if words.get(index + 1).is_some_and(|next| next.text == "then") {
                words[index + 1].end
            } else {
                words[index].end
            };
            conjunction = Some(ends_after);
            break;
        }
        if words[index].text == "then" {
            conjunction = Some(words[index].end);
            break;
        }
    }

    match (punctuation, conjunction) {
        (Some(punctuation), Some(conjunction)) => punctuation.min(conjunction),
        (Some(punctuation), None) => punctuation,
        (None, Some(conjunction)) => conjunction,
        (None, None) => body.len(),
    }
}

/// Lead-ins that describe the hand-off rather than the work inside it.
fn strip_lead_in(value: &str) -> String {
    let words = words(value);
    // "make it do", "have someone look", and the same with a lead-in before
    // them: "please make it do", "can you have them research".
    let lead_ins: [&[&str]; 5] = [
        &["please"],
        &["go", "ahead", "and"],
        &["can", "you"],
        &["could", "you"],
        &["i", "d", "like", "you", "to"],
    ];
    let verbs = ["make", "have", "get", "ask"];
    let them = ["it", "em", "him", "her", "them", "someone"];
    let work = [
        "do", "look", "research", "check", "find", "dig", "run",
    ];

    let mut index = 0;
    'outer: loop {
        // A lead-in may repeat.
        let mut matched_lead = false;
        for lead_in in lead_ins {
            if words[index..].len() >= lead_in.len()
                && words[index..index + lead_in.len()]
                    .iter()
                    .zip(lead_in)
                    .all(|(word, expected)| word.text == *expected)
            {
                index += lead_in.len();
                matched_lead = true;
                break;
            }
        }
        if !matched_lead {
            break;
        }
        if index >= words.len() {
            break 'outer;
        }
    }

    // The verb, the pronoun and the work.
    if index < words.len() && verbs.contains(&words[index].text.as_str()) {
        index += 1;
    } else {
        return value.trim().to_string();
    }
    if index < words.len() && them.contains(&words[index].text.as_str()) {
        index += 1;
    } else {
        return value.trim().to_string();
    }
    if index < words.len() && words[index].text == "to" {
        index += 1;
    }
    if index < words.len() && work.contains(&words[index].text.as_str()) {
        index += 1;
    } else {
        return value.trim().to_string();
    }

    // Whatever is left, from the end of the last matched word.
    let cut = words.get(index).map(|word| word.start).unwrap_or(value.len());
    value[cut..].trim().to_string()
}

/// "ask a subagent to look into X" has no break after the hand-off: the work
/// starts at the verb, so the clause is a prefix rather than a span.
fn strip_lead_clause(value: &str) -> String {
    let words = words(value);
    let lead_ins: [&[&str]; 5] = [
        &["please"],
        &["can", "you"],
        &["could", "you"],
        &["go", "ahead", "and"],
        &["i", "d", "like", "you", "to"],
    ];
    let verbs = ["ask", "have", "get", "tell"];
    let roles = [
        vec!["sub", "agent"],
        vec!["subagent"],
        vec!["agent"],
        vec!["helper"],
        vec!["researcher"],
        vec!["someone"],
        vec!["them"],
    ];

    let mut index = 0;
    loop {
        let mut matched_lead = false;
        for lead_in in lead_ins {
            if words[index..].len() >= lead_in.len()
                && words[index..index + lead_in.len()]
                    .iter()
                    .zip(lead_in)
                    .all(|(word, expected)| word.text == *expected)
            {
                index += lead_in.len();
                matched_lead = true;
                break;
            }
        }
        if !matched_lead {
            break;
        }
        if index >= words.len() {
            return String::new();
        }
    }

    if index < words.len() && verbs.contains(&words[index].text.as_str()) {
        index += 1;
    } else {
        return value.trim().to_string();
    }
    for article in ["a", "an", "another"] {
        if index < words.len() && words[index].text == article {
            index += 1;
            break;
        }
    }
    let mut matched_role = false;
    for role in &roles {
        if words[index..].len() >= role.len()
            && words[index..index + role.len()]
                .iter()
                .zip(role)
                .all(|(word, expected)| word.text == *expected)
        {
            index += role.len();
            matched_role = true;
            break;
        }
    }
    if !matched_role {
        return value.trim().to_string();
    }
    if index < words.len() && words[index].text == "to" {
        index += 1;
    }

    let cut = words.get(index).map(|word| word.start).unwrap_or(value.len());
    value[cut..].trim().to_string()
}

/// The work the sub-agent is being asked to do.
fn extract_task(body: &str, words: &[Word], phrase: Option<(usize, usize)>) -> String {
    let after_clause = match phrase {
        Some((start, length)) => {
            let end = clause_end(body, words, start + length);
            body[end..].trim().to_string()
        }
        None => String::new(),
    };
    let source = if after_clause.is_empty() { body } else { &after_clause };
    let task = collapse_whitespace(&strip_lead_in(source));
    let task = collapse_whitespace(&strip_lead_clause(&task));
    // A request that is nothing but the hand-off still needs something to do.
    if task.is_empty() {
        collapse_whitespace(body)
    } else {
        task
    }
}

/// One run of whitespace becomes one space.
fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// The name the sub-agent gets: the topic if the task names one, the role the
/// request asked for if it named one, and "Researcher" otherwise.
fn sub_agent_name(task: &str, body: &str) -> String {
    // A proper noun in the task is the topic: "search the stock of SpaceX".
    if let Some(topic) = proper_noun(task) {
        return format!("{topic} researcher");
    }
    // Otherwise keep the role the request asked for rather than inventing one
    // from the first long word in the sentence.
    let words = words(body);
    for role in NAMED_ROLES {
        if words.iter().any(|word| word.text == role) {
            let mut characters = role.chars();
            let first = characters.next().unwrap_or('R').to_uppercase().to_string();
            return format!("{first}{}", characters.as_str());
        }
    }
    "Researcher".to_string()
}

/// A proper noun: a capital letter followed by at least two more word
/// characters.
fn proper_noun(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    for index in 0..bytes.len() {
        if !bytes[index].is_ascii_uppercase() {
            continue;
        }
        // The capital must start a word.
        if index > 0 && (bytes[index - 1].is_ascii_alphanumeric() || bytes[index - 1] == b'.') {
            continue;
        }
        let mut end = index + 1;
        while end < bytes.len()
            && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'.' || bytes[end] == b'-')
        {
            end += 1;
        }
        // A trailing dot or dash is punctuation, not part of the name.
        while end > index + 1 && (bytes[end - 1] == b'.' || bytes[end - 1] == b'-') {
            end -= 1;
        }
        if end - index >= 3 {
            return Some(text[index..end].to_string());
        }
    }
    None
}

/// Decide the plan for a turn.
pub fn plan_turn(body: &str) -> TurnPlan {
    let body = body.trim();
    if body.is_empty() {
        return TurnPlan { subagent: None };
    }
    let words = words(body);
    let phrases = delegation_phrases();
    let Some(phrase) = find_delegation(&words, &phrases) else {
        return TurnPlan { subagent: None };
    };

    let task = extract_task(body, &words, Some(phrase));
    let name = sub_agent_name(&task, body);
    TurnPlan {
        subagent: Some(SubAgentPlan {
            focus: task.clone(),
            system_prompt: [
                format!("You are {name}, a sub-agent Agent spawned for one job."),
                "Answer that job and nothing else.".to_string(),
                "Report what you found as plain prose, and say plainly what you could not establish."
                    .to_string(),
            ]
            .join(" "),
            task,
            name,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ported from `tests/planner.test.ts`. The same five cases, the same
    /// claims.
    #[test]
    fn plans_no_delegation_for_ordinary_conversation() {
        for body in [
            "",
            "   ",
            "hello",
            "what model are you",
            "summarise the notes I attached",
            "I will spin up the kettle afterwards",
        ] {
            let plan = plan_turn(body);
            assert!(
                plan.subagent.is_none(),
                "{body:?} should not have planned a delegation: {plan:?}"
            );
        }
    }

    #[test]
    fn plans_a_delegation_when_a_sub_agent_is_named() {
        let plan = plan_turn("run a sub agent, make em do a search on the stock of SpaceX");
        let subagent = plan.subagent.expect("a delegation is planned");
        assert_eq!(subagent.name, "SpaceX researcher");
        assert!(subagent.task.to_lowercase().contains("spacex"), "{}", subagent.task);
    }

    #[test]
    fn plans_a_delegation_when_the_wording_hands_the_work_off() {
        let plan = plan_turn("delegate this to a researcher and have them find the Q3 figures");
        let subagent = plan.subagent.expect("a delegation is planned");
        assert_eq!(subagent.name, "Researcher");
        assert!(!subagent.focus.is_empty());
        assert!(subagent.system_prompt.contains("sub-agent"));
    }

    #[test]
    fn never_plans_an_empty_task() {
        let plan = plan_turn("please spin up an agent");
        let subagent = plan.subagent.expect("a delegation is planned");
        assert!(!subagent.task.is_empty());
        assert_eq!(subagent.focus, subagent.task);
    }

    #[test]
    fn names_the_sub_agent_after_the_topic_it_was_given() {
        let plan = plan_turn("have someone do a deep dive on Anthropic pricing");
        assert_eq!(
            plan.subagent.expect("a delegation is planned").name,
            "Anthropic researcher"
        );
    }

    /// The decision is one typed value, and swapping the decision out later
    /// changes nothing downstream. The capability the plan needs comes from the
    /// plan, not from the wording.
    #[test]
    fn the_plan_names_the_capability_it_needs() {
        let delegating = plan_turn("delegate this to a researcher");
        assert_eq!(
            delegating.capability(),
            Some(Capability::SubagentSpawn),
            "a delegation spawns a sub-agent"
        );
        let ordinary = plan_turn("hello");
        assert_eq!(ordinary.capability(), None);
    }
}
