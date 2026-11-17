//! Domain error hierarchy for Vessel.
//!
//! All variants format with the canonical `[Vessel<Variant>Error]: ...` prefix.

/// Convenience result alias for Vessel domain and core operations.
pub type VesselResult<T> = Result<T, VesselError>;

/// Canonical domain errors shared across core crates and UI adapters.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VesselError {
    /// Failure while performing an outbound network request.
    #[error("[VesselNetworkError]: {0}")]
    Network(String),

    /// Operation exceeded its configured timeout budget.
    #[error("[VesselTimeoutError]: request timed out")]
    Timeout,

    /// Local database or filesystem persistence failure.
    #[error("[VesselStorageError]: {0}")]
    Storage(String),

    /// Error raised inside a sandboxed QuickJS plugin runtime.
    #[error("[VesselPluginRuntimeError]: plugin '{plugin_id}' error: {message}")]
    PluginRuntime { plugin_id: String, message: String },

    /// Outbound plugin request blocked because the domain is not yet allowed.
    #[error(
        "[VesselDomainBlockedError]: plugin '{plugin_id}' requires permission for domain '{domain}'"
    )]
    DomainBlocked { plugin_id: String, domain: String },

    /// Media playback backend failure.
    #[error("[VesselPlayerError]: {0}")]
    Player(String),

    /// Operation was cancelled via a `CancellationToken`.
    #[error("[VesselCancelledError]: operation cancelled")]
    Cancelled,

    /// Requested item, plugin, or stream could not be found.
    #[error("[VesselNotFoundError]: {0}")]
    NotFound(String),

    /// Invalid caller or plugin input.
    #[error("[VesselInvalidInputError]: {0}")]
    InvalidInput(String),
}

#[cfg(test)]
mod tests {
    use super::{VesselError, VesselResult};

    fn check_domain(allowed: bool) -> VesselResult<&'static str> {
        if allowed {
            Ok("ok")
        } else {
            Err(VesselError::DomainBlocked {
                plugin_id: "anthology".into(),
                domain: "cdn.anthology.example".into(),
            })
        }
    }

    fn propagate(allowed: bool) -> VesselResult<usize> {
        let status = check_domain(allowed)?;
        Ok(status.len())
    }

    #[test]
    fn formats_systematic_error_messages() {
        let cases = [
            (
                VesselError::Network("connection refused".into()),
                "[VesselNetworkError]: connection refused",
            ),
            (
                VesselError::Timeout,
                "[VesselTimeoutError]: request timed out",
            ),
            (
                VesselError::Storage("database locked".into()),
                "[VesselStorageError]: database locked",
            ),
            (
                VesselError::PluginRuntime {
                    plugin_id: "anthology".into(),
                    message: "unexpected token".into(),
                },
                "[VesselPluginRuntimeError]: plugin 'anthology' error: unexpected token",
            ),
            (
                VesselError::DomainBlocked {
                    plugin_id: "anthology".into(),
                    domain: "mirror.anthology.example".into(),
                },
                "[VesselDomainBlockedError]: plugin 'anthology' requires permission for domain 'mirror.anthology.example'",
            ),
            (
                VesselError::Player("mpv exited unexpectedly".into()),
                "[VesselPlayerError]: mpv exited unexpectedly",
            ),
            (
                VesselError::Cancelled,
                "[VesselCancelledError]: operation cancelled",
            ),
            (
                VesselError::NotFound("plugin 'anthology'".into()),
                "[VesselNotFoundError]: plugin 'anthology'",
            ),
            (
                VesselError::InvalidInput("empty query".into()),
                "[VesselInvalidInputError]: empty query",
            ),
        ];

        for (err, expected) in cases {
            assert_eq!(err.to_string(), expected);
        }
    }

    #[test]
    fn supports_equality_and_question_mark_propagation() {
        assert_eq!(propagate(true), Ok(2));
        assert_eq!(
            propagate(false),
            Err(VesselError::DomainBlocked {
                plugin_id: "anthology".into(),
                domain: "cdn.anthology.example".into(),
            })
        );
    }
}
