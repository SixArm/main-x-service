//! Pure rules for **custom workflows** — the configurable task and
//! issue state vocabularies (entity spec §5.9.1 / FR-26). DB-free and
//! exhaustively unit-tested.
//!
//! # The load-bearing rule
//!
//! **Every state declares one of four categories** — `todo`, `active`,
//! `waiting`, `done` — and a state without one is refused at write.
//!
//! The board, the burndown, the timeline and every time-based-analysis
//! figure are computed from what a state *means*, not from its name. A
//! free-text vocabulary with no mapping would silently break all of
//! them: an item in a state nobody classified is an item the
//! flow-efficiency denominator cannot account for, and the burndown
//! cannot tell whether it is finished. Refusing is the same posture the
//! transition log already takes toward an unknown status — refused,
//! never coerced.
//!
//! # Category is not the flow class
//!
//! Deliberately two different classifications, and conflating them
//! would be a real defect:
//!
//! - **Category** (here) is *structural*: has this item started, is it
//!   being worked, is it waiting, is it finished. It answers "is this
//!   done?" for the burndown and the board.
//! - **Flow class** ([`crate::tba`]: value-adding / necessary /
//!   unnecessary) is a *value* judgement used by the VSM figures.
//!
//! A state can be `active` and unnecessary at once — rework is exactly
//! that.
//!
//! # An empty transition set means unconstrained
//!
//! Today's board permits any move between statuses. A workflow that
//! declares no transitions keeps that behaviour, so adopting a custom
//! vocabulary does not silently impose a state machine nobody asked
//! for. Constraint is opt-in.

use serde::{Deserialize, Serialize};

/// What a state means structurally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Not started. Counts as backlog dwell.
    Todo,
    /// Being worked on now.
    Active,
    /// Started, not being worked — blocked or queued.
    Waiting,
    /// Finished. **This is what the burndown counts.**
    Done,
}

impl Category {
    /// The wire token.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::Active => "active",
            Self::Waiting => "waiting",
            Self::Done => "done",
        }
    }

    /// Parse a declared category. **`None` for anything unrecognised**
    /// — never a default, because a defaulted category is precisely the
    /// silent misclassification this whole module exists to prevent.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "todo" => Some(Self::Todo),
            "active" => Some(Self::Active),
            "waiting" => Some(Self::Waiting),
            "done" => Some(Self::Done),
            _ => None,
        }
    }
}

/// One declared state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateDef {
    /// Stable token used on the wire and in the transition log.
    pub key: String,
    /// Human label.
    pub label: String,
    /// The structural meaning. Mandatory.
    pub category: Category,
    /// Work-in-progress cap for this column, if any.
    pub wip_limit: Option<u32>,
    /// Whether new work starts here.
    pub is_initial: bool,
    /// Whether the item leaves the board here.
    pub is_terminal: bool,
}

/// A permitted move.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionDef {
    /// Source state key.
    pub from: String,
    /// Target state key.
    pub to: String,
}

/// A complete workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowDef {
    /// The declared states, in board order.
    pub states: Vec<StateDef>,
    /// Permitted moves. **Empty means unconstrained.**
    pub transitions: Vec<TransitionDef>,
}

/// Why a workflow cannot be registered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Invalid {
    /// A workflow with no states is not a workflow.
    NoStates,
    /// A state key was blank or malformed.
    BadKey(String),
    /// Two states share a key.
    DuplicateKey(String),
    /// A state declared no recognised category.
    MissingCategory(String),
    /// Not exactly one initial state. Carries how many were found.
    InitialStateCount(usize),
    /// No state means "finished", so nothing could ever complete and
    /// the burndown would never fall.
    NoDoneState,
    /// A transition names a state that was not declared.
    UnknownTransitionState(String),
}

/// A state key: lowercase letters, digits and underscores, 1..=32.
fn key_ok(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 32
        && key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Validate a workflow.
///
/// # Errors
/// Returns **every** problem found, not merely the first, so one round
/// trip tells an operator everything wrong with the configuration.
pub fn validate(def: &WorkflowDef) -> Result<(), Vec<Invalid>> {
    let mut problems = Vec::new();

    if def.states.is_empty() {
        problems.push(Invalid::NoStates);
        return Err(problems);
    }

    let mut seen: Vec<&str> = Vec::with_capacity(def.states.len());
    for state in &def.states {
        if !key_ok(&state.key) {
            problems.push(Invalid::BadKey(state.key.clone()));
        }
        if seen.contains(&state.key.as_str()) {
            problems.push(Invalid::DuplicateKey(state.key.clone()));
        }
        seen.push(&state.key);
        if state.label.trim().is_empty() {
            problems.push(Invalid::BadKey(state.key.clone()));
        }
    }

    let initial = def.states.iter().filter(|s| s.is_initial).count();
    if initial != 1 {
        problems.push(Invalid::InitialStateCount(initial));
    }

    if !def.states.iter().any(|s| s.category == Category::Done) {
        problems.push(Invalid::NoDoneState);
    }

    for transition in &def.transitions {
        for endpoint in [&transition.from, &transition.to] {
            if !seen.contains(&endpoint.as_str()) {
                problems.push(Invalid::UnknownTransitionState(endpoint.clone()));
            }
        }
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems)
    }
}

/// Whether `def` permits moving from `from` to `to`.
///
/// An **empty** transition set permits everything, preserving today's
/// unconstrained board. A move to an undeclared state is never
/// permitted, whatever the transition set says.
#[must_use]
pub fn may_transition(def: &WorkflowDef, from: &str, to: &str) -> bool {
    if !def.states.iter().any(|s| s.key == to) {
        return false;
    }
    if def.transitions.is_empty() {
        return true;
    }
    def.transitions.iter().any(|t| t.from == from && t.to == to)
}

/// The category of a declared state, or `None` if undeclared.
#[must_use]
pub fn category_of(def: &WorkflowDef, key: &str) -> Option<Category> {
    def.states.iter().find(|s| s.key == key).map(|s| s.category)
}

/// Whether a state means finished.
#[must_use]
pub fn is_done(def: &WorkflowDef, key: &str) -> bool {
    category_of(def, key) == Some(Category::Done)
}

/// The WIP caps this workflow declares, as `status -> cap`.
#[must_use]
pub fn wip_limits(def: &WorkflowDef) -> std::collections::BTreeMap<String, usize> {
    def.states
        .iter()
        .filter_map(|s| s.wip_limit.map(|cap| (s.key.clone(), cap as usize)))
        .collect()
}

fn state(key: &str, label: &str, category: Category, initial: bool, terminal: bool) -> StateDef {
    StateDef {
        key: key.to_string(),
        label: label.to_string(),
        category,
        wip_limit: None,
        is_initial: initial,
        is_terminal: terminal,
    }
}

/// The built-in **task** workflow — today's vocabulary, expressed as a
/// workflow so a plan with nothing configured behaves exactly as
/// before. Unconstrained transitions, matching the current board.
#[must_use]
pub fn built_in_task() -> WorkflowDef {
    WorkflowDef {
        states: vec![
            state("todo", "To do", Category::Todo, true, false),
            state("in_progress", "In progress", Category::Active, false, false),
            state("in_review", "In review", Category::Active, false, false),
            state("done", "Done", Category::Done, false, true),
            // `blocked` is `waiting`, not `active`: an item nobody can
            // work on is not being worked on, and classifying it active
            // would inflate flow efficiency by counting the wait as
            // work — the single most flattering mistake available here.
            state("blocked", "Blocked", Category::Waiting, false, false),
        ],
        transitions: Vec::new(),
    }
}

/// The built-in **issue** workflow.
#[must_use]
pub fn built_in_issue() -> WorkflowDef {
    WorkflowDef {
        states: vec![
            state("open", "Open", Category::Todo, true, false),
            state("in_progress", "In progress", Category::Active, false, false),
            state("resolved", "Resolved", Category::Done, false, false),
            state("closed", "Closed", Category::Done, false, true),
        ],
        transitions: Vec::new(),
    }
}

/// The **flow classes** a custom vocabulary implies, as `status ->
/// VSM category`, for [`crate::tba`].
///
/// Category and flow class are different questions (see the module
/// note), but a custom vocabulary would otherwise arrive at the
/// time-based-analysis layer with **no** classification at all, because
/// the built-in map is keyed on the built-in status names. A board that
/// renamed `in_progress` to `hacking` would then have no value-adding
/// time — a silently empty figure, which is exactly what this module
/// exists to prevent.
///
/// The derivation is the honest default, not a claim to be the right
/// answer for every board:
///
/// | Category | Flow class | Why |
/// |---|---|---|
/// | `active` | value-adding | somebody is working on it |
/// | `todo` | unnecessary | backlog dwell is inventory |
/// | `waiting` | unnecessary | blocked time is waiting, the classic waste |
/// | `done` | *(omitted)* | a terminal state accrues no dwell to classify |
///
/// A deployment that disagrees — a review column that is genuinely
/// *necessary* rather than value-adding, say — overrides it with
/// `PROJECT_PORTFOLIO_MANAGEMENT_FLOW_CLASSES`, which still wins.
#[must_use]
pub fn default_flow_classes(def: &WorkflowDef) -> std::collections::BTreeMap<String, String> {
    def.states
        .iter()
        .filter_map(|state| {
            let class = match state.category {
                Category::Active => crate::tba::CATEGORY_VALUE_ADDING,
                Category::Todo | Category::Waiting => crate::tba::CATEGORY_UNNECESSARY,
                Category::Done => return None,
            };
            Some((state.key.clone(), class.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An unrecognised category is `None`, never a default — the
    /// regression test against a silently misclassified state.
    #[test]
    fn an_unknown_category_is_refused_not_defaulted() {
        assert_eq!(Category::parse("todo"), Some(Category::Todo));
        assert_eq!(Category::parse(" Done "), Some(Category::Done));
        assert_eq!(Category::parse("in_progress"), None);
        assert_eq!(Category::parse(""), None);
        assert_eq!(Category::parse("nearly_done"), None);
    }

    /// Both built-ins are valid, so a plan with nothing configured is
    /// running a workflow that would pass registration.
    #[test]
    fn the_built_ins_are_valid() {
        assert!(validate(&built_in_task()).is_ok());
        assert!(validate(&built_in_issue()).is_ok());
    }

    /// `blocked` is `waiting`, not `active`. Classifying a blocked item
    /// as active would count waiting as work and inflate flow
    /// efficiency — the most flattering error available here.
    #[test]
    fn blocked_is_waiting_not_active() {
        let def = built_in_task();
        assert_eq!(category_of(&def, "blocked"), Some(Category::Waiting));
        assert_eq!(category_of(&def, "in_progress"), Some(Category::Active));
    }

    /// A workflow with no `done` state is refused: nothing could ever
    /// complete, and the burndown would never fall.
    #[test]
    fn a_workflow_that_can_never_finish_is_refused() {
        let def = WorkflowDef {
            states: vec![
                state("a", "A", Category::Todo, true, false),
                state("b", "B", Category::Active, false, false),
            ],
            transitions: Vec::new(),
        };
        assert_eq!(validate(&def), Err(vec![Invalid::NoDoneState]));
    }

    /// Exactly one initial state — zero means new work has nowhere to
    /// land, two means it is ambiguous.
    #[test]
    fn exactly_one_initial_state() {
        let mut def = built_in_task();
        def.states[1].is_initial = true;
        assert_eq!(validate(&def), Err(vec![Invalid::InitialStateCount(2)]));

        let mut none = built_in_task();
        none.states[0].is_initial = false;
        assert_eq!(validate(&none), Err(vec![Invalid::InitialStateCount(0)]));
    }

    /// Every problem is reported at once, not merely the first.
    #[test]
    fn validation_reports_every_problem() {
        let def = WorkflowDef {
            states: vec![
                state("Bad Key", "x", Category::Todo, false, false),
                state("dup", "y", Category::Active, false, false),
                state("dup", "z", Category::Active, false, false),
            ],
            transitions: vec![TransitionDef {
                from: "dup".to_string(),
                to: "nowhere".to_string(),
            }],
        };
        let problems = validate(&def).unwrap_err();
        assert!(problems.contains(&Invalid::BadKey("Bad Key".to_string())));
        assert!(problems.contains(&Invalid::DuplicateKey("dup".to_string())));
        assert!(problems.contains(&Invalid::InitialStateCount(0)));
        assert!(problems.contains(&Invalid::NoDoneState));
        assert!(problems.contains(&Invalid::UnknownTransitionState("nowhere".to_string())));
    }

    /// An empty transition set is **unconstrained**, so adopting a
    /// custom vocabulary does not silently impose a state machine.
    #[test]
    fn no_declared_transitions_means_any_move() {
        let def = built_in_task();
        assert!(may_transition(&def, "todo", "done"));
        assert!(may_transition(&def, "done", "todo"));
        // But never to a state that was not declared.
        assert!(!may_transition(&def, "todo", "invented"));
    }

    /// Declared transitions constrain, and only the declared ones pass.
    #[test]
    fn declared_transitions_constrain() {
        let mut def = built_in_task();
        def.transitions = vec![
            TransitionDef {
                from: "todo".to_string(),
                to: "in_progress".to_string(),
            },
            TransitionDef {
                from: "in_progress".to_string(),
                to: "done".to_string(),
            },
        ];
        assert!(may_transition(&def, "todo", "in_progress"));
        assert!(may_transition(&def, "in_progress", "done"));
        assert!(!may_transition(&def, "todo", "done"));
        assert!(!may_transition(&def, "done", "todo"));
    }

    /// A custom vocabulary keeps every derived view computable, which
    /// is the whole point of the mandatory category.
    #[test]
    fn a_custom_vocabulary_stays_analysable() {
        let def = WorkflowDef {
            states: vec![
                state("icebox", "Icebox", Category::Todo, true, false),
                state("hacking", "Hacking", Category::Active, false, false),
                state(
                    "awaiting_ci",
                    "Awaiting CI",
                    Category::Waiting,
                    false,
                    false,
                ),
                state("shipped", "Shipped", Category::Done, false, true),
            ],
            transitions: Vec::new(),
        };
        assert!(validate(&def).is_ok());
        // Every derived view can still answer its question.
        assert!(is_done(&def, "shipped"));
        assert!(!is_done(&def, "hacking"));
        assert_eq!(category_of(&def, "awaiting_ci"), Some(Category::Waiting));
        // And an undeclared state is unknown rather than assumed.
        assert_eq!(category_of(&def, "todo"), None);
        assert!(!is_done(&def, "todo"));
    }

    /// WIP caps come off the states, so a custom board keeps its limits.
    #[test]
    fn wip_limits_come_from_the_states() {
        let mut def = built_in_task();
        def.states[1].wip_limit = Some(3);
        let limits = wip_limits(&def);
        assert_eq!(limits.get("in_progress"), Some(&3));
        assert_eq!(limits.len(), 1);
    }

    /// A custom vocabulary arrives at the time-based-analysis layer
    /// with a classification, rather than with none. Without this, a
    /// board that renamed `in_progress` to `hacking` would report no
    /// value-adding time at all — a silently empty figure.
    #[test]
    fn a_custom_vocabulary_implies_its_flow_classes() {
        let def = WorkflowDef {
            states: vec![
                state("icebox", "Icebox", Category::Todo, true, false),
                state("hacking", "Hacking", Category::Active, false, false),
                state(
                    "awaiting_ci",
                    "Awaiting CI",
                    Category::Waiting,
                    false,
                    false,
                ),
                state("shipped", "Shipped", Category::Done, false, true),
            ],
            transitions: Vec::new(),
        };
        let classes = default_flow_classes(&def);
        assert_eq!(
            classes.get("hacking").map(String::as_str),
            Some(crate::tba::CATEGORY_VALUE_ADDING)
        );
        assert_eq!(
            classes.get("icebox").map(String::as_str),
            Some(crate::tba::CATEGORY_UNNECESSARY)
        );
        assert_eq!(
            classes.get("awaiting_ci").map(String::as_str),
            Some(crate::tba::CATEGORY_UNNECESSARY)
        );
        // A terminal state accrues no dwell, so it is not classified.
        assert!(!classes.contains_key("shipped"));
    }

    /// **Where the derivation and the disclosed default disagree**, and
    /// why the caller must let the default win.
    ///
    /// Four categories cannot express "necessary non-value-adding":
    /// the built-in `in_review` is *necessary*, but it is structurally
    /// `active`, so the derivation calls it value-adding. Letting that
    /// through would have raised the flow efficiency of every board
    /// that configured nothing — a measurement moving because of an
    /// unrelated feature. `controllers::tba::classes_for` therefore
    /// overlays [`crate::tba::default_classes`] on top of this.
    ///
    /// This test exists because an earlier version of it checked only
    /// `in_progress`, `todo` and `blocked`, passed, and the regression
    /// reached a running service.
    #[test]
    fn the_derivation_disagrees_with_the_default_on_in_review() {
        let classes = default_flow_classes(&built_in_task());
        assert_eq!(
            classes.get("in_progress").map(String::as_str),
            Some(crate::tba::CATEGORY_VALUE_ADDING)
        );
        assert_eq!(
            classes.get("todo").map(String::as_str),
            Some(crate::tba::CATEGORY_UNNECESSARY)
        );
        assert_eq!(
            classes.get("blocked").map(String::as_str),
            Some(crate::tba::CATEGORY_UNNECESSARY)
        );
        // The disagreement, pinned so nobody "fixes" the overlay away.
        assert_eq!(
            classes.get("in_review").map(String::as_str),
            Some(crate::tba::CATEGORY_VALUE_ADDING),
            "the derivation cannot express `necessary`"
        );
        assert_eq!(
            crate::tba::default_classes()
                .get("in_review")
                .map(String::as_str),
            Some(crate::tba::CATEGORY_NECESSARY),
            "the disclosed default says necessary, and must win"
        );
    }
}
