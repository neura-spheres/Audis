//! The session state machine.
//!
//! One engine drives every mode. Live Caption and Meeting Assistant differ in
//! what they persist and whether the assistant runs, not in how they capture
//! audio, so they share this machine rather than each reimplementing it.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::features::FeatureId;
use crate::language::Language;

/// What the user chose to run.
pub type SessionMode = FeatureId;

/// Where a session is in its life.
///
/// Transitions are checked centrally by [`SessionState::can_transition_to`]
/// rather than at each call site, so an illegal move is impossible to write by
/// accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionState {
    /// Nothing running.
    Idle,
    /// Opening devices and loading the model.
    Starting,
    /// Capturing and recognising.
    Listening,
    /// Capture held, devices still open.
    Paused,
    /// Draining buffers and committing the last segments.
    Stopping,
    /// Finished cleanly.
    Completed,
    /// Stopped because something broke.
    Failed,
}

impl SessionState {
    /// Whether `next` is a legal move from here.
    pub fn can_transition_to(self, next: Self) -> bool {
        use SessionState::*;
        match (self, next) {
            (Idle, Starting) => true,
            (Starting, Listening) => true,
            (Listening, Paused) | (Paused, Listening) => true,
            (Listening | Paused, Stopping) => true,
            (Stopping, Completed) => true,
            // Anything live can fail.
            (Starting | Listening | Paused | Stopping, Failed) => true,
            // A finished session is done; starting again means a new session.
            (Completed | Failed, Idle) => true,
            _ => false,
        }
    }

    /// True while devices are open and audio is flowing or held.
    pub fn is_active(self) -> bool {
        matches!(self, Self::Starting | Self::Listening | Self::Paused)
    }

    /// True when Audis is actually capturing.
    ///
    /// This is what drives the visible listening indicator, so it must be true
    /// only when audio is genuinely being captured.
    pub fn is_capturing(self) -> bool {
        matches!(self, Self::Listening)
    }
}

/// The session the UI renders, carried on `audis://session/state`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    /// Session id, stable for its lifetime.
    pub id: Uuid,
    /// Which mode is running.
    pub mode: SessionMode,
    /// Where it is.
    pub state: SessionState,
    /// Language being recognised.
    pub language: Language,
    /// Milliseconds of captured audio, excluding paused time.
    pub elapsed_ms: u64,
    /// Whether the microphone is being captured.
    pub microphone: bool,
    /// Whether computer audio is being captured.
    pub computer_audio: bool,
    /// Whether captions are visible.
    pub captions_visible: bool,
    /// Whether the assistant is running.
    pub assistant_enabled: bool,
    /// Set when `state` is `Failed`.
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use SessionState::*;

    #[test]
    fn the_happy_path_is_legal() {
        assert!(Idle.can_transition_to(Starting));
        assert!(Starting.can_transition_to(Listening));
        assert!(Listening.can_transition_to(Stopping));
        assert!(Stopping.can_transition_to(Completed));
        assert!(Completed.can_transition_to(Idle));
    }

    #[test]
    fn pause_and_resume_round_trip() {
        assert!(Listening.can_transition_to(Paused));
        assert!(Paused.can_transition_to(Listening));
        assert!(Paused.can_transition_to(Stopping));
    }

    #[test]
    fn any_live_state_can_fail() {
        for state in [Starting, Listening, Paused, Stopping] {
            assert!(
                state.can_transition_to(Failed),
                "{state:?} must be able to fail"
            );
        }
    }

    /// The moves that would lose a session or double-start one.
    #[test]
    fn illegal_moves_are_refused() {
        assert!(!Idle.can_transition_to(Listening), "must not skip Starting");
        assert!(
            !Listening.can_transition_to(Starting),
            "must not restart mid-session"
        );
        assert!(!Idle.can_transition_to(Stopping), "nothing to stop");
        assert!(
            !Completed.can_transition_to(Listening),
            "a finished session is finished"
        );
        assert!(
            !Stopping.can_transition_to(Listening),
            "stopping is one-way"
        );
        assert!(!Failed.can_transition_to(Listening));
    }

    #[test]
    fn a_state_never_transitions_to_itself() {
        for state in [
            Idle, Starting, Listening, Paused, Stopping, Completed, Failed,
        ] {
            assert!(!state.can_transition_to(state), "{state:?} to itself");
        }
    }

    /// The indicator must be lit only while audio is genuinely captured.
    #[test]
    fn only_listening_counts_as_capturing() {
        assert!(Listening.is_capturing());
        for state in [Idle, Starting, Paused, Stopping, Completed, Failed] {
            assert!(
                !state.is_capturing(),
                "{state:?} must not claim to be capturing"
            );
        }
    }

    #[test]
    fn active_covers_every_state_holding_a_device() {
        for state in [Starting, Listening, Paused] {
            assert!(state.is_active());
        }
        for state in [Idle, Stopping, Completed, Failed] {
            assert!(!state.is_active());
        }
    }
}
