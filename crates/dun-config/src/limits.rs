#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub editable_file_soft_limit_bytes: u64,
    pub line_display_soft_limit_bytes: usize,
}

impl Limits {
    pub fn validate(self) -> Result<(), LimitsError> {
        if self.editable_file_soft_limit_bytes == 0 {
            return Err(LimitsError::EditableFileSoftLimitZero);
        }

        if self.line_display_soft_limit_bytes == 0 {
            return Err(LimitsError::LineDisplaySoftLimitZero);
        }

        Ok(())
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            editable_file_soft_limit_bytes: 16 * 1024 * 1024,
            line_display_soft_limit_bytes: 16 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitsError {
    EditableFileSoftLimitZero,
    LineDisplaySoftLimitZero,
}
