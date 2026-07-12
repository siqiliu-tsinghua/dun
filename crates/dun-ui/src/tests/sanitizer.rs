use super::support::*;

const WIDTH: u16 = 512;
const HEIGHT: u16 = 24;
const CONTROL_ATTACKS: [&str; 8] = [
    "\u{1b}[2J",
    "\u{1b}[H",
    "\u{1b}]0;pwned\u{7}",
    "\u{1b}]52;c;SGVsbG8=\u{7}",
    "\u{1b}Ppayload\u{1b}\\",
    "\u{9b}2J",
    "\u{7}",
    "\r\u{8}",
];
const RTL_OVERRIDE: &str = "\u{202e}";

/// Invisible format characters. A zero-width space inside an identifier makes
/// `ad\u{200b}min` read as `admin`; the tag block smuggles arbitrary ASCII past
/// the eye. They draw nothing, so only the emitted bytes can prove they are gone.
const ZERO_WIDTH_ATTACKS: [&str; 5] = [
    "\u{200b}",  // ZERO WIDTH SPACE
    "\u{200d}",  // ZERO WIDTH JOINER
    "\u{feff}",  // BOM
    "\u{00ad}",  // SOFT HYPHEN -- vim misses this one
    "\u{e0067}", // TAG LATIN SMALL LETTER G -- vim misses these too
];
const POISONED_FIELDS: [&str; 10] = [
    "BUFFER_BODY",
    "WINDOW_TITLE",
    "STATUS_LEFT",
    "STATUS_RIGHT",
    "PLUGIN_INDICATOR",
    "OVERLAY_TITLE",
    "OVERLAY_LINE",
    "OVERLAY_INPUT",
    "OVERLAY_LIST_ENTRY",
    "OVERLAY_BUTTON",
];

fn poison(field: &str) -> String {
    let mut text = format!("{field}_BEGIN:");
    for attack in CONTROL_ATTACKS {
        text.push_str(attack);
        text.push('|');
    }
    text.push_str(RTL_OVERRIDE);
    for attack in ZERO_WIDTH_ATTACKS {
        text.push_str(attack);
    }
    text.push_str(&format!(":{field}_END"));
    text
}

fn emitted_poisoned_frame() -> String {
    let mut workspace = Workspace::new_untitled();
    workspace.window_mut(WindowId(1)).unwrap().title = poison("WINDOW_TITLE");
    let buffer = TextBuffer::from_text_with_kind(BufferKind::Untitled, &poison("BUFFER_BODY"));
    let buffer_view = BufferView::new(BufferId(1), &buffer);
    let shell = UiShell::default();
    let mut frame = shell.frame_for_workspace(
        &workspace,
        Rect::new(0, 0, WIDTH, HEIGHT - 2),
        &[buffer_view],
    );
    frame.status.left = poison("STATUS_LEFT");
    frame.status.right = poison("STATUS_RIGHT");
    frame.status.plugin = Some(PluginIndicator {
        text: poison("PLUGIN_INDICATOR"),
        alert: true,
    });
    frame.overlay = Some(UiOverlay::file_dialog(
        poison("OVERLAY_TITLE"),
        vec![poison("OVERLAY_LINE")],
        poison("OVERLAY_INPUT"),
        0,
        vec![poison("OVERLAY_LIST_ENTRY")],
        Some(0),
        vec![poison("OVERLAY_BUTTON")],
    ));

    let mut renderer = SurfaceRenderer::new();
    let emitted = renderer.render(&shell, &frame, WIDTH, HEIGHT).bytes;
    let emitted = String::from_utf8(emitted).expect("surface emitter output must be UTF-8");
    strip_renderer_sequences(&emitted)
}

fn strip_renderer_sequences(emitted: &str) -> String {
    let bytes = emitted.as_bytes();
    let mut stripped = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if let Some(length) = renderer_sequence_length(&bytes[index..]) {
            index += length;
        } else {
            stripped.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(stripped).expect("removing ASCII renderer sequences preserves UTF-8")
}

fn renderer_sequence_length(bytes: &[u8]) -> Option<usize> {
    if !bytes.starts_with(b"\x1b[") {
        return None;
    }

    let mut final_index = 2;
    while bytes
        .get(final_index)
        .is_some_and(|byte| byte.is_ascii_digit() || *byte == b';')
    {
        final_index += 1;
    }
    let parameters = bytes.get(2..final_index)?;
    let valid_parameters = || {
        !parameters.is_empty()
            && parameters
                .split(|byte| *byte == b';')
                .all(|parameter| !parameter.is_empty() && parameter.iter().all(u8::is_ascii_digit))
    };

    match bytes.get(final_index) {
        Some(b'H') if valid_parameters() && parameters.split(|byte| *byte == b';').count() == 2 => {
            Some(final_index + 1)
        }
        Some(b'm')
            if valid_parameters()
                && parameters.split(|byte| *byte == b';').next() == Some(b"0") =>
        {
            Some(final_index + 1)
        }
        _ => None,
    }
}

fn assert_every_field_reached_emitter(emitted: &str) {
    for field in POISONED_FIELDS {
        assert!(
            emitted.contains(&format!("{field}_BEGIN:"))
                && emitted.contains(&format!(":{field}_END")),
            "poisoned {field} did not reach the emitted byte stream"
        );
    }
}

fn fields_containing(emitted: &str, needle: &str) -> Vec<&'static str> {
    POISONED_FIELDS
        .into_iter()
        .filter(|field| {
            let start_marker = format!("{field}_BEGIN:");
            let end_marker = format!(":{field}_END");
            let Some(start) = emitted.find(&start_marker) else {
                return false;
            };
            let value = &emitted[start + start_marker.len()..];
            let Some(end) = value.find(&end_marker) else {
                return false;
            };
            value[..end].contains(needle)
        })
        .collect()
}

#[test]
fn renderer_sequence_stripper_preserves_attacker_sequences() {
    let attacks = CONTROL_ATTACKS.concat();
    let emitted = format!("\u{1b}[1;1H\u{1b}[0;39;49m{attacks}");

    assert_eq!(strip_renderer_sequences(&emitted), attacks);
}

#[test]
fn poisoned_frame_emits_no_attacker_control_sequences() {
    let emitted = emitted_poisoned_frame();
    assert_every_field_reached_emitter(&emitted);

    for attack in CONTROL_ATTACKS {
        assert!(
            !emitted.contains(attack),
            "attacker sequence survived surface emission: {attack:?} in {emitted:?}"
        );
    }
    assert!(
        !emitted.chars().any(char::is_control),
        "attacker-controlled terminal control reached emitted bytes: {emitted:?}"
    );
    assert!(!emitted.contains('\u{1b}'), "raw ESC reached emitted bytes");
    assert!(
        !emitted.contains('\u{9b}'),
        "raw C1 CSI reached emitted bytes"
    );
    assert!(!emitted.contains('\u{7}'), "raw BEL reached emitted bytes");
}

// Known hole: UTF-8 sanitization currently passes the bidi-formatting scalar
// U+202E through every text path below, so a hostile name can disguise text.
#[test]
fn poisoned_frame_emits_no_rtl_override() {
    let emitted = emitted_poisoned_frame();
    assert_every_field_reached_emitter(&emitted);
    let vulnerable_fields = fields_containing(&emitted, RTL_OVERRIDE);

    assert!(
        vulnerable_fields.is_empty(),
        "U+202E RTL override reached terminal from fields {vulnerable_fields:?}"
    );
}

/// The zero-width half of the same hole. U+202E was found first because it
/// reorders what you see; these draw nothing at all, which is worse -- there is
/// no visual tell whatsoever. vim escapes most of them and misses SOFT HYPHEN
/// and the tag block; dun escapes all of them, so none may reach the terminal.
#[test]
fn poisoned_frame_emits_no_zero_width_formatting() {
    let emitted = emitted_poisoned_frame();
    assert_every_field_reached_emitter(&emitted);

    for attack in ZERO_WIDTH_ATTACKS {
        let vulnerable_fields = fields_containing(&emitted, attack);
        assert!(
            vulnerable_fields.is_empty(),
            "U+{:04X} reached the terminal from fields {vulnerable_fields:?}",
            attack.chars().next().map(u32::from).unwrap_or_default()
        );
    }
}
