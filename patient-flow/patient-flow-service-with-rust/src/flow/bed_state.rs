//! The bed state machine (spec `bed-management.md`), as a pure
//! transition function: `(current state, transition, context) →
//! outcome | error`. No clock, no database — controllers persist the
//! outcome and stamp `state_since`.

use serde::{Deserialize, Serialize};

/// A bed's operational state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BedState {
    /// Ready for allocation.
    Available,
    /// Held for an allocated bed request.
    Reserved,
    /// One active stay occupies it.
    Occupied,
    /// Vacated, waiting for the domestic team.
    AwaitingClean,
    /// Being cleaned.
    Cleaning,
    /// Closed to use (with a reason).
    Closed,
}

impl BedState {
    /// The stored token for this state.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Reserved => "reserved",
            Self::Occupied => "occupied",
            Self::AwaitingClean => "awaiting_clean",
            Self::Cleaning => "cleaning",
            Self::Closed => "closed",
        }
    }

    /// Parse a stored token.
    ///
    /// # Errors
    ///
    /// When `s` is not a bed-state token.
    pub fn parse(s: &str) -> Result<Self, TransitionError> {
        match s {
            "available" => Ok(Self::Available),
            "reserved" => Ok(Self::Reserved),
            "occupied" => Ok(Self::Occupied),
            "awaiting_clean" => Ok(Self::AwaitingClean),
            "cleaning" => Ok(Self::Cleaning),
            "closed" => Ok(Self::Closed),
            _ => Err(TransitionError::UnknownState(s.to_string())),
        }
    }
}

/// A requested bed transition (spec `bed-management.md` table).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transition {
    /// available → reserved (bed-request allocation).
    Allocate,
    /// reserved → available (allocation cancelled / expired).
    Release,
    /// available | reserved → occupied (stay placed).
    Admit,
    /// occupied → `awaiting_clean` (physical) / available (virtual).
    /// `infectious` marks the departing stay as carrying an uncleared
    /// contact/droplet/airborne flag ⇒ sets `deep_clean_required`.
    Vacate {
        /// Departing stay had an uncleared transmissible flag.
        infectious: bool,
    },
    /// `awaiting_clean` → cleaning.
    CleanStart,
    /// cleaning → available. A deep-clean-required bed needs the
    /// explicit deep-clean confirmation, not the routine one.
    CleanComplete {
        /// The completed clean was a confirmed deep clean.
        deep_clean_done: bool,
    },
    /// Any non-occupied state → closed (with a reason token).
    Close {
        /// One of [`crate::flow::tokens::CLOSURE_REASONS`].
        reason: String,
    },
    /// closed → available (or `awaiting_clean` when a deep clean is
    /// still owed).
    Reopen,
}

/// Why a transition was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionError {
    /// The stored state token is not part of the vocabulary.
    UnknownState(String),
    /// The transition is not legal from the current state.
    Illegal {
        /// The current state's token.
        from: &'static str,
        /// The requested transition, described.
        transition: String,
    },
    /// `Close` without a valid closure reason.
    BadClosureReason(String),
    /// A deep-clean-required bed completed only a routine clean.
    DeepCleanOutstanding,
}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownState(s) => write!(f, "unknown bed state {s:?}"),
            Self::Illegal { from, transition } => {
                write!(f, "cannot {transition} a bed in state {from:?}")
            }
            Self::BadClosureReason(r) => write!(f, "unknown closure reason {r:?}"),
            Self::DeepCleanOutstanding => {
                write!(f, "bed requires a deep clean; routine clean-complete refused")
            }
        }
    }
}

impl std::error::Error for TransitionError {}

/// Context the transition needs about the bed.
#[derive(Debug, Clone, Copy, Default)]
pub struct BedContext {
    /// A virtual-ward slot (skips the cleaning cycle, PF-D8).
    pub is_virtual: bool,
    /// A deep clean is owed from a previous infectious occupant.
    pub deep_clean_required: bool,
}

/// The successful result of a transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The next state.
    pub state: BedState,
    /// The (possibly updated) deep-clean-owed flag.
    pub deep_clean_required: bool,
    /// The closure reason to store (`Some` only when closed).
    pub closure_reason: Option<String>,
}

/// Apply `transition` to a bed in `state` under `ctx`.
///
/// # Errors
///
/// [`TransitionError::Illegal`] when the move is not in the state
/// machine; [`TransitionError::BadClosureReason`] on an unknown close
/// reason; [`TransitionError::DeepCleanOutstanding`] when a routine
/// clean-complete is attempted on a deep-clean-owed bed.
pub fn apply(state: BedState, transition: &Transition, ctx: BedContext) -> Result<Outcome, TransitionError> {
    use BedState as S;
    let illegal = |t: &str| TransitionError::Illegal {
        from: state.token(),
        transition: t.to_string(),
    };
    let ok = |next: BedState, deep: bool, reason: Option<String>| {
        Ok(Outcome {
            state: next,
            deep_clean_required: deep,
            closure_reason: reason,
        })
    };
    match transition {
        Transition::Allocate => match state {
            S::Available => ok(S::Reserved, ctx.deep_clean_required, None),
            _ => Err(illegal("allocate")),
        },
        Transition::Release => match state {
            S::Reserved => ok(S::Available, ctx.deep_clean_required, None),
            _ => Err(illegal("release")),
        },
        Transition::Admit => match state {
            S::Available | S::Reserved => ok(S::Occupied, ctx.deep_clean_required, None),
            _ => Err(illegal("admit")),
        },
        Transition::Vacate { infectious } => match state {
            S::Occupied if ctx.is_virtual => ok(S::Available, false, None),
            S::Occupied => ok(
                S::AwaitingClean,
                ctx.deep_clean_required || *infectious,
                None,
            ),
            _ => Err(illegal("vacate")),
        },
        Transition::CleanStart => match state {
            S::AwaitingClean => ok(S::Cleaning, ctx.deep_clean_required, None),
            _ => Err(illegal("start cleaning")),
        },
        Transition::CleanComplete { deep_clean_done } => match state {
            S::Cleaning if ctx.deep_clean_required && !deep_clean_done => {
                Err(TransitionError::DeepCleanOutstanding)
            }
            S::Cleaning => ok(S::Available, false, None),
            _ => Err(illegal("complete cleaning")),
        },
        Transition::Close { reason } => {
            if !crate::flow::tokens::is_token(crate::flow::tokens::CLOSURE_REASONS, reason) {
                return Err(TransitionError::BadClosureReason(reason.clone()));
            }
            match state {
                S::Occupied => Err(illegal("close")),
                _ => ok(S::Closed, ctx.deep_clean_required, Some(reason.clone())),
            }
        }
        Transition::Reopen => match state {
            S::Closed if ctx.deep_clean_required => ok(S::AwaitingClean, true, None),
            S::Closed => ok(S::Available, false, None),
            _ => Err(illegal("reopen")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> BedContext {
        BedContext::default()
    }

    /// Every legal transition in the spec table produces its target
    /// state.
    #[test]
    fn legal_transitions() {
        let cases: Vec<(BedState, Transition, BedState)> = vec![
            (BedState::Available, Transition::Allocate, BedState::Reserved),
            (BedState::Reserved, Transition::Release, BedState::Available),
            (BedState::Available, Transition::Admit, BedState::Occupied),
            (BedState::Reserved, Transition::Admit, BedState::Occupied),
            (
                BedState::Occupied,
                Transition::Vacate { infectious: false },
                BedState::AwaitingClean,
            ),
            (BedState::AwaitingClean, Transition::CleanStart, BedState::Cleaning),
            (
                BedState::Cleaning,
                Transition::CleanComplete { deep_clean_done: false },
                BedState::Available,
            ),
        ];
        for (from, t, want) in cases {
            let got = apply(from, &t, ctx()).unwrap();
            assert_eq!(got.state, want, "{from:?} --{t:?}--> {want:?}");
        }
    }

    /// Every illegal (state, transition) pair is refused, naming the
    /// current state — exhaustive over the non-close transitions.
    #[test]
    fn illegal_transitions_are_refused() {
        use BedState as S;
        let all = [S::Available, S::Reserved, S::Occupied, S::AwaitingClean, S::Cleaning, S::Closed];
        let legal_from = |t: &Transition| -> Vec<BedState> {
            match t {
                Transition::Allocate => vec![S::Available],
                Transition::Release => vec![S::Reserved],
                Transition::Admit => vec![S::Available, S::Reserved],
                Transition::Vacate { .. } => vec![S::Occupied],
                Transition::CleanStart => vec![S::AwaitingClean],
                Transition::CleanComplete { .. } => vec![S::Cleaning],
                Transition::Reopen => vec![S::Closed],
                Transition::Close { .. } => unreachable!(),
            }
        };
        let transitions = [
            Transition::Allocate,
            Transition::Release,
            Transition::Admit,
            Transition::Vacate { infectious: false },
            Transition::CleanStart,
            Transition::CleanComplete { deep_clean_done: false },
            Transition::Reopen,
        ];
        for t in &transitions {
            for from in all {
                let legal = legal_from(t).contains(&from);
                assert_eq!(
                    apply(from, t, ctx()).is_ok(),
                    legal,
                    "{from:?} --{t:?}--> legality mismatch"
                );
            }
        }
    }

    /// Close is legal from every non-occupied state and refused when
    /// occupied; an unknown reason is refused.
    #[test]
    fn close_rules() {
        use BedState as S;
        let close = Transition::Close { reason: "infection".to_string() };
        for from in [S::Available, S::Reserved, S::AwaitingClean, S::Cleaning, S::Closed] {
            let out = apply(from, &close, ctx()).unwrap();
            assert_eq!(out.state, S::Closed);
            assert_eq!(out.closure_reason.as_deref(), Some("infection"));
        }
        assert!(apply(S::Occupied, &close, ctx()).is_err());
        let bad = Transition::Close { reason: "meteor".to_string() };
        assert_eq!(
            apply(S::Available, &bad, ctx()),
            Err(TransitionError::BadClosureReason("meteor".to_string()))
        );
    }

    /// An infectious vacate sets deep-clean-required; a routine
    /// clean-complete is then refused until the deep clean is
    /// confirmed, which clears the flag.
    #[test]
    fn deep_clean_propagation() {
        let out = apply(BedState::Occupied, &Transition::Vacate { infectious: true }, ctx()).unwrap();
        assert_eq!(out.state, BedState::AwaitingClean);
        assert!(out.deep_clean_required);
        let deep = BedContext { is_virtual: false, deep_clean_required: true };
        assert_eq!(
            apply(BedState::Cleaning, &Transition::CleanComplete { deep_clean_done: false }, deep),
            Err(TransitionError::DeepCleanOutstanding)
        );
        let done = apply(BedState::Cleaning, &Transition::CleanComplete { deep_clean_done: true }, deep).unwrap();
        assert_eq!(done.state, BedState::Available);
        assert!(!done.deep_clean_required);
    }

    /// Virtual slots skip the cleaning cycle: vacate returns straight
    /// to available and never owes a deep clean (PF-D8).
    #[test]
    fn virtual_slots_skip_cleaning() {
        let virt = BedContext { is_virtual: true, deep_clean_required: false };
        let out = apply(BedState::Occupied, &Transition::Vacate { infectious: true }, virt).unwrap();
        assert_eq!(out.state, BedState::Available);
        assert!(!out.deep_clean_required);
    }

    /// Reopening a closed bed goes to awaiting-clean when a deep clean
    /// is owed, else straight to available.
    #[test]
    fn reopen_respects_owed_deep_clean() {
        let owed = BedContext { is_virtual: false, deep_clean_required: true };
        let out = apply(BedState::Closed, &Transition::Reopen, owed).unwrap();
        assert_eq!(out.state, BedState::AwaitingClean);
        assert!(out.deep_clean_required);
        let clear = apply(BedState::Closed, &Transition::Reopen, ctx()).unwrap();
        assert_eq!(clear.state, BedState::Available);
    }

    /// Token round-trip for every state.
    #[test]
    fn state_tokens_round_trip() {
        for s in [
            BedState::Available,
            BedState::Reserved,
            BedState::Occupied,
            BedState::AwaitingClean,
            BedState::Cleaning,
            BedState::Closed,
        ] {
            assert_eq!(BedState::parse(s.token()).unwrap(), s);
        }
        assert!(BedState::parse("hovering").is_err());
    }
}
