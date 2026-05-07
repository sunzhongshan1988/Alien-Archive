#![allow(dead_code)]

use runtime::{Rect, Vec2};

#[derive(Clone, Copy, Debug, Default)]
pub struct Insets {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Insets {
    pub const ZERO: Self = Self::all(0.0);

    pub const fn all(value: f32) -> Self {
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }

    pub const fn xy(x: f32, y: f32) -> Self {
        Self {
            left: x,
            right: x,
            top: y,
            bottom: y,
        }
    }

    pub const fn new(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }

    pub fn scaled(self, scale: f32) -> Self {
        Self {
            left: self.left * scale,
            right: self.right * scale,
            top: self.top * scale,
            bottom: self.bottom * scale,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug)]
pub enum Justify {
    Start,
    Center,
    End,
    SpaceBetween,
}

#[derive(Clone, Copy, Debug)]
pub struct Stack {
    axis: Axis,
    padding: Insets,
    gap: f32,
    align: Align,
    justify: Justify,
}

impl Stack {
    pub const fn horizontal() -> Self {
        Self {
            axis: Axis::Horizontal,
            padding: Insets::ZERO,
            gap: 0.0,
            align: Align::Stretch,
            justify: Justify::Start,
        }
    }

    pub const fn vertical() -> Self {
        Self {
            axis: Axis::Vertical,
            padding: Insets::ZERO,
            gap: 0.0,
            align: Align::Stretch,
            justify: Justify::Start,
        }
    }

    pub const fn padding(mut self, padding: Insets) -> Self {
        self.padding = padding;
        self
    }

    pub const fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub const fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    pub const fn justify(mut self, justify: Justify) -> Self {
        self.justify = justify;
        self
    }

    pub fn even(self, rect: Rect, count: usize) -> Vec<Rect> {
        if count == 0 {
            return Vec::new();
        }

        let inner = inset(rect, self.padding);
        let gap_total = self.gap * count.saturating_sub(1) as f32;
        match self.axis {
            Axis::Horizontal => {
                let width = ((inner.size.x - gap_total) / count as f32).max(0.0);
                self.fixed_main(inner, &vec![width; count], None)
            }
            Axis::Vertical => {
                let height = ((inner.size.y - gap_total) / count as f32).max(0.0);
                self.fixed_main(inner, &vec![height; count], None)
            }
        }
    }

    pub fn fixed_main(self, rect: Rect, main_sizes: &[f32], cross_size: Option<f32>) -> Vec<Rect> {
        if main_sizes.is_empty() {
            return Vec::new();
        }

        let inner = inset(rect, self.padding);
        let total_main = main_sizes.iter().sum::<f32>() + self.gap * (main_sizes.len() - 1) as f32;
        let available_main = match self.axis {
            Axis::Horizontal => inner.size.x,
            Axis::Vertical => inner.size.y,
        };
        let slack = (available_main - total_main).max(0.0);
        let between_gap = match self.justify {
            Justify::SpaceBetween if main_sizes.len() > 1 => {
                self.gap + slack / (main_sizes.len() - 1) as f32
            }
            _ => self.gap,
        };
        let mut cursor = match self.justify {
            Justify::Start | Justify::SpaceBetween => 0.0,
            Justify::Center => slack * 0.5,
            Justify::End => slack,
        };

        let mut rects = Vec::with_capacity(main_sizes.len());
        for main in main_sizes {
            let cross_available = match self.axis {
                Axis::Horizontal => inner.size.y,
                Axis::Vertical => inner.size.x,
            };
            let cross = cross_size
                .unwrap_or(cross_available)
                .min(cross_available)
                .max(0.0);
            let cross_offset = match self.align {
                Align::Start | Align::Stretch => 0.0,
                Align::Center => (cross_available - cross) * 0.5,
                Align::End => cross_available - cross,
            };

            let rect = match self.axis {
                Axis::Horizontal => Rect::new(
                    Vec2::new(inner.origin.x + cursor, inner.origin.y + cross_offset),
                    Vec2::new(
                        *main,
                        if matches!(self.align, Align::Stretch) {
                            inner.size.y
                        } else {
                            cross
                        },
                    ),
                ),
                Axis::Vertical => Rect::new(
                    Vec2::new(inner.origin.x + cross_offset, inner.origin.y + cursor),
                    Vec2::new(
                        if matches!(self.align, Align::Stretch) {
                            inner.size.x
                        } else {
                            cross
                        },
                        *main,
                    ),
                ),
            };
            rects.push(rect);
            cursor += *main + between_gap;
        }

        rects
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Grid {
    columns: usize,
    rows: usize,
    padding: Insets,
    gap: Vec2,
}

impl Grid {
    pub const fn new(columns: usize, rows: usize) -> Self {
        Self {
            columns,
            rows,
            padding: Insets::ZERO,
            gap: Vec2::ZERO,
        }
    }

    pub const fn padding(mut self, padding: Insets) -> Self {
        self.padding = padding;
        self
    }

    pub const fn gap(mut self, x: f32, y: f32) -> Self {
        self.gap = Vec2::new(x, y);
        self
    }

    pub fn cells(self, rect: Rect) -> Vec<Rect> {
        if self.columns == 0 || self.rows == 0 {
            return Vec::new();
        }

        let inner = inset(rect, self.padding);
        let total_gap_x = self.gap.x * self.columns.saturating_sub(1) as f32;
        let total_gap_y = self.gap.y * self.rows.saturating_sub(1) as f32;
        let cell_w = ((inner.size.x - total_gap_x) / self.columns as f32).max(0.0);
        let cell_h = ((inner.size.y - total_gap_y) / self.rows as f32).max(0.0);

        let mut cells = Vec::with_capacity(self.columns * self.rows);
        for row in 0..self.rows {
            for col in 0..self.columns {
                cells.push(Rect::new(
                    Vec2::new(
                        inner.origin.x + col as f32 * (cell_w + self.gap.x),
                        inner.origin.y + row as f32 * (cell_h + self.gap.y),
                    ),
                    Vec2::new(cell_w, cell_h),
                ));
            }
        }
        cells
    }
}

pub fn inset(rect: Rect, insets: Insets) -> Rect {
    Rect::new(
        Vec2::new(rect.origin.x + insets.left, rect.origin.y + insets.top),
        Vec2::new(
            (rect.size.x - insets.left - insets.right).max(0.0),
            (rect.size.y - insets.top - insets.bottom).max(0.0),
        ),
    )
}
