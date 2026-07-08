use super::support::*;

#[test]
fn limits_reject_zero_values() {
    assert_eq!(
        Limits {
            line_display_soft_limit_bytes: 0,
            ..Limits::default()
        }
        .validate(),
        Err(LimitsError::LineDisplaySoftLimitZero)
    );
}
