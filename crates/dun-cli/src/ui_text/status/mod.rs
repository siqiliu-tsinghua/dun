use super::TextKey;

mod command;
mod command_output;
mod edit;
mod file;
mod prompt;
mod search;
mod window;

pub(crate) use command::*;
pub(crate) use command_output::*;
pub(crate) use edit::*;
pub(crate) use file::*;
pub(crate) use prompt::*;
pub(crate) use search::*;
pub(crate) use window::*;

#[cfg(test)]
const MODULES: [&[TextKey]; 7] = [
    window::ALL,
    file::ALL,
    edit::ALL,
    search::ALL,
    prompt::ALL,
    command::ALL,
    command_output::ALL,
];

#[cfg(test)]
const ALL_LEN: usize = window::ALL.len()
    + file::ALL.len()
    + edit::ALL.len()
    + search::ALL.len()
    + prompt::ALL.len()
    + command::ALL.len()
    + command_output::ALL.len();

#[cfg(test)]
const ALL_ARRAY: [TextKey; ALL_LEN] = {
    let mut all = [("", ""); ALL_LEN];
    let mut index = 0;
    let mut module_index = 0;
    while module_index < MODULES.len() {
        let mut key_index = 0;
        while key_index < MODULES[module_index].len() {
            all[index] = MODULES[module_index][key_index];
            index += 1;
            key_index += 1;
        }
        module_index += 1;
    }
    all
};

/// Every status key above, for the translation-completeness test.
#[cfg(test)]
pub(crate) const ALL: &[TextKey] = &ALL_ARRAY;
