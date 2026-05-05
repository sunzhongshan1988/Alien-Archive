#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ToolKind {
    Select,
    Brush,
    Bucket,
    Rectangle,
    Erase,
    Eyedropper,
    Collision,
    Zone,
    Pan,
    Zoom,
}

impl ToolKind {
    pub(crate) const ALL: [Self; 10] = [
        Self::Select,
        Self::Brush,
        Self::Bucket,
        Self::Rectangle,
        Self::Erase,
        Self::Eyedropper,
        Self::Collision,
        Self::Zone,
        Self::Pan,
        Self::Zoom,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Select => "选择",
            Self::Brush => "画笔",
            Self::Bucket => "油漆桶",
            Self::Rectangle => "矩形",
            Self::Erase => "橡皮",
            Self::Eyedropper => "吸管",
            Self::Collision => "碰撞",
            Self::Zone => "区域",
            Self::Pan => "平移",
            Self::Zoom => "缩放",
        }
    }
}
