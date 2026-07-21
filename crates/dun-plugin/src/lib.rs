#![forbid(unsafe_code)]

//! Host-neutral Dun Plugin Protocol client (`docs/plugin-protocol.md`).
//!
//! Framed stdio transport, hand-rolled JSON (a deliberate trusted-computing-
//! base decision — see the protocol doc), envelope/role/policy types, output
//! validation, and timeout/cancel/crash handling. Grown from the measured
//! `spike/plugin-client-size` prototype; wired into `dun-cli` via the
//! worker-thread host lifecycle in `crates/dun-cli/src/plugins.rs`.
//! Protocol v0 has no LoadPlugin/UnloadPlugin message kinds by design: one
//! plugin per host process, so host lifecycle is plugin lifecycle (the
//! editor-level `plugin load`/`unload` commands manage it). Revisit only if
//! multi-plugin hosts become a real need.

pub mod capability;
pub mod client;
pub mod frame;
pub mod json;
pub mod keybinding;
pub mod menu;
pub mod proto;
pub mod validate;

pub use capability::{Capability, GrantedCapabilities};
pub use client::{HostClient, PluginError};
pub use keybinding::{PluginChord, PluginKeybinding};
pub use menu::{LabelSet, PluginActionKind, PluginMenu, PluginMenuItem};
pub use proto::{Policy, Role, TrustClass};
pub use validate::{InputSnapshot, StreamChunk, StyleId, StyleSpan};
