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
        .filter(|translated| indexed_template_is_valid(translated, args.len()))
        .unwrap_or(key.1);
    substitute(template, args)
}

/// The validated translated template for a key, when the caller needs its
/// own English fallback (e.g. English singular/plural branching).
pub(crate) fn tr_template(catalog: &TextCatalog, key: TextKey, arg_count: usize) -> Option<&str> {
    catalog
        .get(key.0)
        .filter(|translated| indexed_template_is_valid(translated, arg_count))
}

/// How many arguments a template consumes.
///
/// English defaults use positional `{}`, filled left to right. A translation
/// may instead use **indexed** `{0}`, `{1}`, … to put the arguments in an
/// order its language needs — Japanese and Korean are verb-final, and Russian
/// word order is free, so a template like `Find: {}/{} matches: {}` cannot be
/// said naturally with the English order forced on it. An indexed template's
/// arity is its highest index plus one.
///
/// The two forms do not mix: a template is positional or indexed, never both.
///
/// Only the shipped-translation validator needs the count as a number; the
/// runtime asks `indexed_template_is_valid` instead.
#[cfg(test)]
pub(crate) fn placeholder_count(template: &str) -> usize {
    match indexed_placeholders(template) {
        Some(indices) => indices.iter().copied().max().map_or(0, |max| max + 1),
        None => template.matches("{}").count(),
    }
}

/// The `{N}` indices a template uses, or `None` if it uses none. A template
/// that mixes `{}` with `{N}` is rejected (returns `None` with a stray `{}`
/// left in place), so validation catches it rather than rendering nonsense.
fn indexed_placeholders(template: &str) -> Option<Vec<usize>> {
    let mut indices = Vec::new();
    let mut rest = template;
    while let Some((_, tail)) = rest.split_once('{') {
        let (body, after) = tail.split_once('}')?;
        if !body.is_empty() {
            indices.push(body.parse().ok()?);
        }
        rest = after;
    }
    (!indices.is_empty()).then_some(indices)
}

/// Every index in `0..arity` is used at least once, and no index is out of
/// range. This is what the shipped-translation validator checks: an indexed
/// template that skips an argument would silently drop a runtime value.
pub(crate) fn indexed_template_is_valid(template: &str, arity: usize) -> bool {
    match indexed_placeholders(template) {
        None => template.matches("{}").count() == arity,
        Some(indices) => {
            !template.contains("{}")
                && (0..arity).all(|index| indices.contains(&index))
                && indices.iter().all(|index| *index < arity)
        }
    }
}

pub(crate) fn substitute(template: &str, args: &[&str]) -> String {
    if let Some(_indices) = indexed_placeholders(template) {
        return substitute_indexed(template, args);
    }

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

fn substitute_indexed(template: &str, args: &[&str]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    while let Some((head, tail)) = rest.split_once('{') {
        out.push_str(head);
        let Some((body, after)) = tail.split_once('}') else {
            out.push('{');
            rest = tail;
            continue;
        };
        match body.parse::<usize>().ok().and_then(|index| args.get(index)) {
            Some(arg) => out.push_str(arg),
            // An out-of-range index cannot happen for a validated template;
            // if one somehow arrives, drop it rather than render `{7}`.
            None => {}
        }
        rest = after;
    }
    out.push_str(rest);
    out
}
