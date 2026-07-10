#![forbid(unsafe_code)]

//! Host-neutral Dun Plugin Protocol client (`docs/plugin-protocol.md`).
//!
//! Framed stdio transport, hand-rolled JSON (a deliberate trusted-computing-
//! base decision — see the protocol doc), envelope/role/policy types, output
//! validation, and timeout/cancel/crash handling. Grown from the measured
//! `spike/plugin-client-size` prototype. Not yet wired into `dun-cli`;
//! config integration and role application land with the protocol-client
//! stage. LoadPlugin/UnloadPlugin message kinds are still pending.

pub mod client;
pub mod frame;
pub mod json;
pub mod proto;
pub mod validate;

pub use client::{HostClient, PluginError};
pub use proto::{Policy, Role, TrustClass};
pub use validate::{InputSnapshot, StyleId, StyleSpan};
