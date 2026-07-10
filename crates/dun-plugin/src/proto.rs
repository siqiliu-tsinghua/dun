//! Protocol envelope, message kinds, roles, trust classes, and policy.

use std::fmt;
use std::time::Duration;

use crate::json::{self, Json};

pub const PROTOCOL_VERSION: u64 = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageKind {
    Hello,
    HelloAck,
    Request,
    Response,
    Diagnostic,
    CancelRequest,
    Error,
    Shutdown,
}

impl MessageKind {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Hello => "hello",
            Self::HelloAck => "hello-ack",
            Self::Request => "request",
            Self::Response => "response",
            Self::Diagnostic => "diagnostic",
            Self::CancelRequest => "cancel-request",
            Self::Error => "error",
            Self::Shutdown => "shutdown",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "hello" => Some(Self::Hello),
            "hello-ack" => Some(Self::HelloAck),
            "request" => Some(Self::Request),
            "response" => Some(Self::Response),
            "diagnostic" => Some(Self::Diagnostic),
            "cancel-request" => Some(Self::CancelRequest),
            "error" => Some(Self::Error),
            "shutdown" => Some(Self::Shutdown),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    SyntaxHighlight,
}

impl Role {
    pub const fn id(self) -> &'static str {
        match self {
            Self::SyntaxHighlight => "syntax-highlight",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "syntax-highlight" => Some(Self::SyntaxHighlight),
            _ => None,
        }
    }
}

/// Protocol compatibility is not a security claim; unknown trust classes are
/// rejected at handshake instead of being modeled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrustClass {
    PureSandbox,
    UserTrustedExternal,
}

impl TrustClass {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "pure-sandbox" => Some(Self::PureSandbox),
            "user-trusted-external" => Some(Self::UserTrustedExternal),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Policy {
    pub max_frame_bytes: usize,
    pub max_snapshot_lines: usize,
    pub max_spans: usize,
    pub max_stderr_bytes: usize,
    pub max_diagnostics: usize,
    pub timeout: Duration,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            max_frame_bytes: 256 * 1024,
            max_snapshot_lines: 512,
            max_spans: 4096,
            max_stderr_bytes: 8 * 1024,
            max_diagnostics: 16,
            timeout: Duration::from_millis(2_000),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Envelope {
    pub kind: MessageKind,
    pub request_id: u64,
    pub plugin_id: String,
    pub role: Option<Role>,
    pub revision: Option<u64>,
    pub payload: Json,
}

impl Envelope {
    pub fn to_json_bytes(&self) -> Vec<u8> {
        let mut fields = vec![
            ("v".to_string(), json::num(PROTOCOL_VERSION)),
            ("kind".to_string(), json::str(self.kind.id())),
            ("request_id".to_string(), json::num(self.request_id)),
            ("plugin_id".to_string(), json::str(&self.plugin_id)),
        ];
        if let Some(role) = self.role {
            fields.push(("role".to_string(), json::str(role.id())));
        }
        if let Some(revision) = self.revision {
            fields.push(("revision".to_string(), json::num(revision)));
        }
        fields.push(("payload".to_string(), self.payload.clone()));
        json::to_string(&Json::Obj(fields)).into_bytes()
    }

    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let value = json::parse(bytes).map_err(ProtocolError::Json)?;
        let version = field_u64(&value, "v")?;
        if version != PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedVersion(version));
        }
        let kind = MessageKind::from_id(field_str(&value, "kind")?)
            .ok_or(ProtocolError::BadField("kind"))?;
        let request_id = field_u64(&value, "request_id")?;
        let plugin_id = field_str(&value, "plugin_id")?.to_string();
        let role = match value.get("role") {
            None | Some(Json::Null) => None,
            Some(role_value) => {
                let role_id = role_value.as_str().ok_or(ProtocolError::BadField("role"))?;
                Some(Role::from_id(role_id).ok_or(ProtocolError::BadField("role"))?)
            }
        };
        let revision = match value.get("revision") {
            None | Some(Json::Null) => None,
            Some(revision_value) => Some(
                revision_value
                    .as_u64()
                    .ok_or(ProtocolError::BadField("revision"))?,
            ),
        };
        let payload = value.get("payload").cloned().unwrap_or(Json::Null);
        Ok(Self {
            kind,
            request_id,
            plugin_id,
            role,
            revision,
            payload,
        })
    }
}

fn field_str<'a>(value: &'a Json, key: &'static str) -> Result<&'a str, ProtocolError> {
    value
        .get(key)
        .and_then(Json::as_str)
        .ok_or(ProtocolError::MissingField(key))
}

fn field_u64(value: &Json, key: &'static str) -> Result<u64, ProtocolError> {
    value
        .get(key)
        .and_then(Json::as_u64)
        .ok_or(ProtocolError::MissingField(key))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    Json(crate::json::JsonError),
    MissingField(&'static str),
    BadField(&'static str),
    UnsupportedVersion(u64),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "{error}"),
            Self::MissingField(field) => write!(formatter, "missing field '{field}'"),
            Self::BadField(field) => write!(formatter, "invalid field '{field}'"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported protocol version {version}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_roundtrips_with_optional_fields() {
        let envelope = Envelope {
            kind: MessageKind::Request,
            request_id: 42,
            plugin_id: "spike".to_string(),
            role: Some(Role::SyntaxHighlight),
            revision: Some(7),
            payload: json::obj([("language", json::str("rust"))]),
        };
        let bytes = envelope.to_json_bytes();
        let parsed = Envelope::from_json_bytes(&bytes).expect("parses");
        assert_eq!(parsed.kind, MessageKind::Request);
        assert_eq!(parsed.request_id, 42);
        assert_eq!(parsed.plugin_id, "spike");
        assert_eq!(parsed.role, Some(Role::SyntaxHighlight));
        assert_eq!(parsed.revision, Some(7));
        assert_eq!(
            parsed.payload.get("language").and_then(Json::as_str),
            Some("rust")
        );
    }

    #[test]
    fn rejects_wrong_version_and_unknown_kind() {
        assert!(matches!(
            Envelope::from_json_bytes(
                br#"{"v":9,"kind":"hello","request_id":0,"plugin_id":"x","payload":null}"#
            ),
            Err(ProtocolError::UnsupportedVersion(9))
        ));
        assert!(matches!(
            Envelope::from_json_bytes(
                br#"{"v":0,"kind":"nope","request_id":0,"plugin_id":"x","payload":null}"#
            ),
            Err(ProtocolError::BadField("kind"))
        ));
    }
}
