use super::*;

impl Workspace {
    pub fn focus_direction(
        &mut self,
        direction: Direction,
        area: Rect,
    ) -> Result<WindowId, WorkspaceError> {
        let layouts = self.resolved_layout(area);
        let Some(current) = layouts.iter().find(|layout| layout.id == self.focused) else {
            return Err(WorkspaceError::FocusMissing);
        };

        let next = layouts
            .iter()
            .filter(|layout| layout.id != self.focused)
            .filter_map(|layout| {
                neighbor_score(direction, current.rect, layout.rect).map(|score| (score, layout.id))
            })
            .min_by_key(|(score, _)| *score)
            .map(|(_, id)| id)
            .ok_or(WorkspaceError::NoNeighbor)?;

        self.focused = next;
        Ok(next)
    }

    pub fn window_at(&self, area: Rect, x: u16, y: u16) -> Option<WindowId> {
        self.resolved_layout(area)
            .into_iter()
            .find(|layout| layout.rect.contains(x, y))
            .map(|layout| layout.id)
    }

    pub fn focus_at(&mut self, area: Rect, x: u16, y: u16) -> Option<WindowId> {
        let window_id = self.window_at(area, x, y)?;
        if self.window(window_id).is_err() {
            return None;
        }

        self.focused = window_id;
        Some(window_id)
    }
}

fn neighbor_score(direction: Direction, current: Rect, candidate: Rect) -> Option<(u16, u16)> {
    match direction {
        Direction::Left if candidate.right() <= current.x => Some((
            current.x.saturating_sub(candidate.right()),
            center_delta(current.center_y(), candidate.center_y()),
        )),
        Direction::Right if candidate.x >= current.right() => Some((
            candidate.x.saturating_sub(current.right()),
            center_delta(current.center_y(), candidate.center_y()),
        )),
        Direction::Up if candidate.bottom() <= current.y => Some((
            current.y.saturating_sub(candidate.bottom()),
            center_delta(current.center_x(), candidate.center_x()),
        )),
        Direction::Down if candidate.y >= current.bottom() => Some((
            candidate.y.saturating_sub(current.bottom()),
            center_delta(current.center_x(), candidate.center_x()),
        )),
        _ => None,
    }
}

fn center_delta(a: i32, b: i32) -> u16 {
    a.abs_diff(b).min(u16::MAX as u32) as u16
}
