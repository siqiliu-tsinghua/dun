//! The inward capability vocabulary: typed, validated channels into
//! `dun`-owned objects (`docs/plugin-protocol.md`, "Capability Model").
//!
//! `dun` grants no ambient authority; the protocol carries no
//! authority-request fields. A plugin may only produce output for the
//! capabilities its role holds, and every capability is bounded and validated
//! on each message. A [`Role`] is a `dun`-defined *named bundle* of
//! capabilities: config declares roles, and `dun` expands each to its
//! capability set, subject to the trust gate below.

use crate::proto::{Role, TrustClass};

/// A single permission to touch one `dun`-owned object through a typed,
/// validated channel. The wire carries capabilities as kebab-case string ids;
/// unknown ids are rejected rather than modeled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Receive a bounded read snapshot of an editor buffer.
    BufferRead,
    /// Receive bounded chunks of a `dun`-managed output stream.
    StreamRead,
    /// Return style spans over a buffer's rendering; never mutates buffer text.
    OverlayWrite,
    /// Write validated, sanitized text into the plugin's own surface.
    SurfaceWrite,
    /// Create and destroy the plugin's own windows (bounded, own-only).
    Window,
    /// Own an editable scratch buffer window and submit its text on execute.
    ScratchInput,
    /// Contribute one top-level menu subtree that dispatches to the host.
    Menu,
    /// Reserve one leader prefix key and bind chords beneath it.
    Keybinding,
}

impl Capability {
    /// Every capability in the v0 vocabulary, in declaration order.
    pub const ALL: [Capability; 8] = [
        Self::BufferRead,
        Self::StreamRead,
        Self::OverlayWrite,
        Self::SurfaceWrite,
        Self::Window,
        Self::ScratchInput,
        Self::Menu,
        Self::Keybinding,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::BufferRead => "buffer-read",
            Self::StreamRead => "stream-read",
            Self::OverlayWrite => "overlay-write",
            Self::SurfaceWrite => "surface-write",
            Self::Window => "window",
            Self::ScratchInput => "scratch-input",
            Self::Menu => "menu",
            Self::Keybinding => "keybinding",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "buffer-read" => Some(Self::BufferRead),
            "stream-read" => Some(Self::StreamRead),
            "overlay-write" => Some(Self::OverlayWrite),
            "surface-write" => Some(Self::SurfaceWrite),
            "window" => Some(Self::Window),
            "scratch-input" => Some(Self::ScratchInput),
            "menu" => Some(Self::Menu),
            "keybinding" => Some(Self::Keybinding),
            _ => None,
        }
    }

    /// The least trust class `dun` requires before it will grant this
    /// capability. Read-only and validated-write capabilities are safe to
    /// auto-grant to a `pure-sandbox` runtime; UI-invasive and code-executing
    /// capabilities require an explicitly `user-trusted-external` host plus a
    /// config opt-in (the opt-in is enforced where the grant is wired, not
    /// here).
    pub const fn min_trust(self) -> TrustClass {
        match self {
            Self::BufferRead | Self::StreamRead | Self::OverlayWrite | Self::SurfaceWrite => {
                TrustClass::PureSandbox
            }
            Self::Window | Self::ScratchInput | Self::Menu | Self::Keybinding => {
                TrustClass::UserTrustedExternal
            }
        }
    }

    /// Whether a host of the given trust class clears the trust gate for this
    /// capability. `pure-sandbox`-eligible capabilities are grantable to any
    /// trust class; the rest require `user-trusted-external`.
    pub fn trust_gate_cleared(self, trust: TrustClass) -> bool {
        match self.min_trust() {
            TrustClass::PureSandbox => true,
            TrustClass::UserTrustedExternal => trust == TrustClass::UserTrustedExternal,
        }
    }
}

/// The capability set `dun` has granted a host: the union of its declared
/// roles' bundles, kept only where the trust gate clears. Output and UI actions
/// are checked against this set; nothing outside it is honored. Stored as a
/// small bitmask rather than a `HashSet` to stay frugal in the size-budgeted
/// binary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GrantedCapabilities {
    bits: u16,
}

impl GrantedCapabilities {
    const fn mask(capability: Capability) -> u16 {
        1u16 << (capability as u16)
    }

    /// Grant each desired capability that clears the trust gate for `trust`.
    /// Capabilities the trust class denies are dropped, never honored.
    pub fn grant(desired: impl IntoIterator<Item = Capability>, trust: TrustClass) -> Self {
        let mut bits = 0u16;
        for capability in desired {
            if capability.trust_gate_cleared(trust) {
                bits |= Self::mask(capability);
            }
        }
        Self { bits }
    }

    /// Grant the capabilities of the host's declared roles, trust-gated.
    pub fn for_roles(roles: &[Role], trust: TrustClass) -> Self {
        Self::grant(
            roles
                .iter()
                .flat_map(|role| role.capabilities().iter().copied()),
            trust,
        )
    }

    /// Whether this capability is granted, i.e. its channel may be used.
    pub const fn holds(self, capability: Capability) -> bool {
        self.bits & Self::mask(capability) != 0
    }
}

impl Role {
    /// The capability bundle this role expands to. Roles are `dun`-defined
    /// named bundles, so this mapping is authoritative and local; v0 ships
    /// `SyntaxHighlight` only (`{ buffer-read, overlay-write }`).
    pub const fn capabilities(self) -> &'static [Capability] {
        match self {
            Role::SyntaxHighlight => &[Capability::BufferRead, Capability::OverlayWrite],
        }
    }

    /// The capability whose validated `Response` this role's request returns.
    /// The role-request path validates output through this capability's
    /// validator (v0: `SyntaxHighlight` returns overlay spans); each later
    /// slice slots its role's output capability here.
    pub const fn output_capability(self) -> Capability {
        match self {
            Role::SyntaxHighlight => Capability::OverlayWrite,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_ids_round_trip_and_reject_unknown() {
        for capability in Capability::ALL {
            assert_eq!(Capability::from_id(capability.id()), Some(capability));
        }
        assert_eq!(Capability::from_id("run-command"), None);
        assert_eq!(Capability::from_id(""), None);
    }

    #[test]
    fn trust_tier_matches_the_documented_split() {
        let auto = [
            Capability::BufferRead,
            Capability::StreamRead,
            Capability::OverlayWrite,
            Capability::SurfaceWrite,
        ];
        let gated = [
            Capability::Window,
            Capability::ScratchInput,
            Capability::Menu,
            Capability::Keybinding,
        ];
        for capability in auto {
            assert_eq!(capability.min_trust(), TrustClass::PureSandbox);
            assert!(capability.trust_gate_cleared(TrustClass::PureSandbox));
            assert!(capability.trust_gate_cleared(TrustClass::UserTrustedExternal));
        }
        for capability in gated {
            assert_eq!(capability.min_trust(), TrustClass::UserTrustedExternal);
            assert!(!capability.trust_gate_cleared(TrustClass::PureSandbox));
            assert!(capability.trust_gate_cleared(TrustClass::UserTrustedExternal));
        }
    }

    #[test]
    fn syntax_highlight_bundle_is_read_plus_overlay() {
        assert_eq!(
            Role::SyntaxHighlight.capabilities(),
            &[Capability::BufferRead, Capability::OverlayWrite]
        );
    }

    #[test]
    fn syntax_highlight_output_is_overlay_write() {
        assert_eq!(
            Role::SyntaxHighlight.output_capability(),
            Capability::OverlayWrite
        );
    }

    #[test]
    fn grant_expands_roles_and_enforces_the_trust_gate() {
        let sandbox = TrustClass::PureSandbox;
        let trusted = TrustClass::UserTrustedExternal;

        // A bundle of auto-grantable capabilities is granted under any trust.
        let highlight = GrantedCapabilities::for_roles(&[Role::SyntaxHighlight], sandbox);
        assert!(highlight.holds(Capability::BufferRead));
        assert!(highlight.holds(Capability::OverlayWrite));
        assert!(!highlight.holds(Capability::Window));

        // Gated capabilities are dropped under pure-sandbox, kept under
        // user-trusted-external — the enforcement that keeps a host from
        // gaining a UI channel it was not trusted for.
        let mixed = [Capability::BufferRead, Capability::Window, Capability::Menu];
        let denied = GrantedCapabilities::grant(mixed, sandbox);
        assert!(denied.holds(Capability::BufferRead));
        assert!(!denied.holds(Capability::Window));
        assert!(!denied.holds(Capability::Menu));

        let allowed = GrantedCapabilities::grant(mixed, trusted);
        assert!(allowed.holds(Capability::Window));
        assert!(allowed.holds(Capability::Menu));
    }
}
