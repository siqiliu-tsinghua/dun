// Syntax-highlight fixture for the acceptance gallery (docs/real-terminal-
// acceptance.md, Section D). Not compiled by anything: it is opened as TEXT,
// so a highlight host has something that exercises all five of dun's style
// classes — keyword, comment, string, number, emphasis.
//
// The language hint dun sends a host is just the lowercased file extension
// (crates/dun-cli/src/plugins.rs:942), hence the real .rs name.

use std::collections::HashMap;

/// Doc comment — syntect marks these differently from a line comment.
pub struct Budget {
    pub limit_bytes: u64,
    pub label: String,
}

impl Budget {
    pub fn new(label: &str) -> Self {
        Self {
            limit_bytes: 1_048_576, // the hard size budget
            label: label.to_string(),
        }
    }

    /// Returns how many bytes are left, saturating at zero.
    pub fn remaining(&self, used: u64) -> u64 {
        self.limit_bytes.saturating_sub(used)
    }

    pub fn describe(&self, used: u64) -> String {
        let left = self.remaining(used);
        if left == 0 {
            return format!("{}: OVER BUDGET by {} bytes", self.label, used - self.limit_bytes);
        }
        format!("{}: {} of {} used, {} left", self.label, used, self.limit_bytes, left)
    }
}

fn main() {
    let mut measured: HashMap<&str, u64> = HashMap::new();
    measured.insert("macos", 677_940);
    measured.insert("debian", 760_112);

    let budget = Budget::new("dun");
    for (platform, used) in &measured {
        println!("{platform:>8}  {}", budget.describe(*used));
    }

    // Escapes and unicode in string literals:
    let tricky = "tab:\t quote:\" backslash:\\ newline:\\n 中文 Ыдентификатор";
    assert!(!tricky.is_empty(), "fixture string must not be empty");

    let hex = 0xFF_u8;
    let float = 3.141_592_65_f64;
    let boolean = true;
    println!("{hex} {float} {boolean}");
}
