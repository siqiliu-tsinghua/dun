//! Fixed UI chrome and status text with catalog keys (docs/i18n.md, slices 3–4).
//!
//! Every translatable dialog/overlay/window-title/status string is declared
//! once as a `(catalog key, English default)` pair, so call sites cannot invent
//! keys ad hoc and the translation-completeness test can enumerate the full set.

use dun_config::TextCatalog;

mod chrome;
mod status;

pub(crate) use chrome::*;
pub(crate) use status::*;

pub(crate) type TextKey = (&'static str, &'static str);

/// Every UI-text key, for the translation-completeness test.
#[cfg(test)]
const ALL_ARRAY: [TextKey; chrome::ALL.len() + status::ALL.len()] = {
    let mut all = [("", ""); chrome::ALL.len() + status::ALL.len()];
    let mut index = 0;
    while index < chrome::ALL.len() {
        all[index] = chrome::ALL[index];
        index += 1;
    }
    let mut status_index = 0;
    while status_index < status::ALL.len() {
        all[index] = status::ALL[status_index];
        index += 1;
        status_index += 1;
    }
    all
};

#[cfg(test)]
pub(crate) const ALL: &[TextKey] = &ALL_ARRAY;

/// Translate a fixed string.
pub(crate) fn tr(catalog: &TextCatalog, key: TextKey) -> &str {
    catalog.get(key.0).unwrap_or(key.1)
}

/// Translate a `{}`-template and substitute arguments left to right. A
/// translated template whose placeholder count does not match the argument
/// count is ignored in favor of the English template: a translation mistake
/// must never drop or duplicate runtime values.
pub(crate) fn tr_fmt(catalog: &TextCatalog, key: TextKey, args: &[&str]) -> String {
    let template = catalog
        .get(key.0)
        .filter(|translated| placeholder_count(translated) == args.len())
        .unwrap_or(key.1);
    substitute(template, args)
}

/// The validated translated template for a key, when the caller needs its
/// own English fallback (e.g. English singular/plural branching).
pub(crate) fn tr_template(catalog: &TextCatalog, key: TextKey, arg_count: usize) -> Option<&str> {
    catalog
        .get(key.0)
        .filter(|translated| placeholder_count(translated) == arg_count)
}

pub(crate) fn placeholder_count(template: &str) -> usize {
    template.matches("{}").count()
}

pub(crate) fn substitute(template: &str, args: &[&str]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    for arg in args {
        match rest.split_once("{}") {
            Some((head, tail)) => {
                out.push_str(head);
                out.push_str(arg);
                rest = tail;
            }
            None => break,
        }
    }
    out.push_str(rest);
    out
}
