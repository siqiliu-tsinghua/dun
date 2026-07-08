use std::env;

pub(crate) fn detect_terminal_profile() -> dun_term::TerminalProfile {
    let term = env::var("TERM").ok();
    let colorterm = env::var("COLORTERM").ok();
    let lang = env::var("LANG").ok();
    let lc_ctype = env::var("LC_CTYPE").ok();
    let no_color = env::var_os("NO_COLOR").is_some();

    dun_term::TerminalProfile::from_capabilities(
        term.as_deref(),
        colorterm.as_deref(),
        lang.as_deref(),
        lc_ctype.as_deref(),
        no_color,
    )
}
