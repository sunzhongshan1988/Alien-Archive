use crate::{Rect, Vec2};

pub fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.origin.x < b.right()
        && a.right() > b.origin.x
        && a.origin.y < b.bottom()
        && a.bottom() > b.origin.y
}

pub fn point_in_rect(point: Vec2, rect: Rect) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.right()
        && point.y >= rect.origin.y
        && point.y <= rect.bottom()
}
