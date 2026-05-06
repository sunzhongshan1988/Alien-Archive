use std::{collections::HashMap, path::Path};

use anyhow::Result;
use runtime::{Button, Color, InputState, Rect, Renderer, SceneCommand, Vec2};
use rusttype::Font;

use crate::ui::text::{TextSprite, draw_text, load_ui_font, upload_text};

use super::{GameContext, Language, RenderContext, Scene, SceneId};

const EXPLORER_PORTRAIT_TEXTURE_ID: &str = "profile.explorer_portrait";
const EXPLORER_PORTRAIT_PATH: &str = "assets/images/ui/profile/explorer_portrait.png";
const PROFILE_WIDTH: f32 = 1132.0;
const PROFILE_HEIGHT: f32 = 612.0;
const HEADER_HEIGHT: f32 = 78.0;
const OUTER_PADDING: f32 = 24.0;
const CONTENT_GAP: f32 = 18.0;
const LEFT_PANEL_SIZE: Vec2 = Vec2::new(420.0, 494.0);
const CENTER_PANEL_SIZE: Vec2 = Vec2::new(328.0, 494.0);
const RIGHT_PANEL_SIZE: Vec2 = Vec2::new(300.0, 494.0);
const BAR_WIDTH: f32 = 188.0;
const BAR_HEIGHT: f32 = 10.0;
const SCORE_PIP_SIZE: f32 = 14.0;
const SCORE_PIP_GAP: f32 = 6.0;

#[derive(Clone, Copy)]
struct LocalizedText {
    english: &'static str,
    chinese: &'static str,
}

impl LocalizedText {
    const fn new(english: &'static str, chinese: &'static str) -> Self {
        Self { english, chinese }
    }

    fn get(self, language: Language) -> &'static str {
        match language {
            Language::Chinese => self.chinese,
            Language::English => self.english,
        }
    }
}

#[derive(Clone, Copy)]
struct PercentStat {
    id: &'static str,
    label: LocalizedText,
    value: u32,
    max: u32,
}

#[derive(Clone, Copy)]
struct ScoreStat {
    id: &'static str,
    label: LocalizedText,
    value: u32,
}

#[derive(Clone)]
pub(super) struct ProfileOverview {
    pub(super) callsign: &'static str,
    pub(super) role: &'static str,
    pub(super) id_line: &'static str,
    pub(super) vital_stats: Vec<ProfilePercentView>,
    pub(super) core_stats: Vec<ProfileScoreView>,
    pub(super) research_stats: Vec<ProfilePercentView>,
}

#[derive(Clone, Copy)]
pub(super) struct ProfilePercentView {
    pub(super) label: &'static str,
    pub(super) value: u32,
    pub(super) max: u32,
}

#[derive(Clone, Copy)]
pub(super) struct ProfileScoreView {
    pub(super) label: &'static str,
    pub(super) value: u32,
}

const VITAL_STATS: &[PercentStat] = &[
    PercentStat {
        id: "health",
        label: LocalizedText::new("Health", "生命"),
        value: 86,
        max: 100,
    },
    PercentStat {
        id: "stamina",
        label: LocalizedText::new("Stamina", "体力"),
        value: 72,
        max: 100,
    },
    PercentStat {
        id: "suit",
        label: LocalizedText::new("Suit Integrity", "外骨骼完整度"),
        value: 91,
        max: 100,
    },
    PercentStat {
        id: "load",
        label: LocalizedText::new("Carry Load", "负重"),
        value: 37,
        max: 60,
    },
];

const CORE_STATS: &[ScoreStat] = &[
    ScoreStat {
        id: "survival",
        label: LocalizedText::new("Survival", "生存"),
        value: 7,
    },
    ScoreStat {
        id: "mobility",
        label: LocalizedText::new("Mobility", "机动"),
        value: 6,
    },
    ScoreStat {
        id: "scanning",
        label: LocalizedText::new("Scanning", "扫描"),
        value: 8,
    },
    ScoreStat {
        id: "harvesting",
        label: LocalizedText::new("Harvesting", "采集"),
        value: 6,
    },
    ScoreStat {
        id: "analysis",
        label: LocalizedText::new("Analysis", "解析"),
        value: 7,
    },
];

const RESEARCH_STATS: &[PercentStat] = &[
    PercentStat {
        id: "bio",
        label: LocalizedText::new("Bio Samples", "生物样本"),
        value: 42,
        max: 100,
    },
    PercentStat {
        id: "mineral",
        label: LocalizedText::new("Mineral Survey", "矿物调查"),
        value: 55,
        max: 100,
    },
    PercentStat {
        id: "ruin",
        label: LocalizedText::new("Ruin Tech", "遗迹科技"),
        value: 31,
        max: 100,
    },
    PercentStat {
        id: "data",
        label: LocalizedText::new("Data Analysis", "数据解析"),
        value: 68,
        max: 100,
    },
];

const RESISTANCE_STATS: &[PercentStat] = &[
    PercentStat {
        id: "spores",
        label: LocalizedText::new("Toxic Spores", "毒性孢子"),
        value: 40,
        max: 100,
    },
    PercentStat {
        id: "heat",
        label: LocalizedText::new("Heat", "高温"),
        value: 35,
        max: 100,
    },
    PercentStat {
        id: "radiation",
        label: LocalizedText::new("Radiation", "辐射"),
        value: 28,
        max: 100,
    },
    PercentStat {
        id: "oxygen",
        label: LocalizedText::new("Low Oxygen", "低氧"),
        value: 62,
        max: 100,
    },
];

#[derive(Default)]
struct ProfileText {
    language: Option<Language>,
    title: Option<TextSprite>,
    callsign: Option<TextSprite>,
    role: Option<TextSprite>,
    id_line: Option<TextSprite>,
    status_header: Option<TextSprite>,
    core_header: Option<TextSprite>,
    research_header: Option<TextSprite>,
    resistance_header: Option<TextSprite>,
    vital_labels: HashMap<&'static str, TextSprite>,
    vital_values: HashMap<&'static str, TextSprite>,
    core_labels: HashMap<&'static str, TextSprite>,
    core_values: HashMap<&'static str, TextSprite>,
    research_labels: HashMap<&'static str, TextSprite>,
    research_values: HashMap<&'static str, TextSprite>,
    resistance_labels: HashMap<&'static str, TextSprite>,
    resistance_values: HashMap<&'static str, TextSprite>,
}

pub struct ProfileScene {
    language: Language,
    text: ProfileText,
}

impl ProfileScene {
    pub fn new(language: Language) -> Self {
        Self {
            language,
            text: ProfileText::default(),
        }
    }

    fn draw_profile(&self, ctx: &mut RenderContext<'_>) {
        let viewport = ctx.renderer.screen_size();
        let layout = ProfileLayout::new(viewport);

        ctx.renderer.draw_rect(
            screen_rect(viewport, 0.0, 0.0, viewport.x, viewport.y),
            Color::rgba(0.0, 0.0, 0.0, 0.78),
        );

        self.draw_shell(ctx.renderer, viewport, &layout);
        self.draw_identity(ctx.renderer, viewport, &layout);
        self.draw_core(ctx.renderer, viewport, &layout);
        self.draw_research(ctx.renderer, viewport, &layout);
    }

    fn draw_shell(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &ProfileLayout) {
        draw_panel(
            renderer,
            layout.root_panel,
            Color::rgba(0.014, 0.023, 0.031, 0.98),
            Color::rgba(0.25, 0.38, 0.50, 0.82),
            layout.scale,
        );
        draw_corner_brackets(
            renderer,
            layout.root_panel,
            22.0 * layout.scale,
            2.0 * layout.scale,
            Color::rgba(0.28, 0.88, 1.0, 0.95),
        );
        renderer.draw_rect(
            Rect::new(
                layout.header.origin,
                Vec2::new(layout.header.size.x, 1.0 * layout.scale),
            ),
            Color::rgba(0.31, 0.92, 1.0, 0.82),
        );
        renderer.draw_rect(
            Rect::new(
                Vec2::new(
                    layout.header.origin.x + 22.0 * layout.scale,
                    layout.header.bottom() - 12.0 * layout.scale,
                ),
                Vec2::new(170.0 * layout.scale, 2.0 * layout.scale),
            ),
            Color::rgba(0.90, 0.70, 0.32, 0.90),
        );

        if let Some(title) = &self.text.title {
            draw_text(
                renderer,
                title,
                viewport,
                screen_x(viewport, layout.header.origin.x + 24.0 * layout.scale),
                screen_y(viewport, layout.header.origin.y + 16.0 * layout.scale),
                Color::rgba(0.90, 1.0, 0.98, 1.0),
            );
        }
    }

    fn draw_identity(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &ProfileLayout) {
        draw_panel(
            renderer,
            layout.left_panel,
            Color::rgba(0.017, 0.027, 0.035, 0.94),
            Color::rgba(0.18, 0.29, 0.39, 0.78),
            layout.scale,
        );
        renderer.draw_rect(
            Rect::new(
                Vec2::new(
                    layout.left_panel.origin.x + 20.0 * layout.scale,
                    layout.left_panel.origin.y + 22.0 * layout.scale,
                ),
                Vec2::new(96.0 * layout.scale, 2.0 * layout.scale),
            ),
            Color::rgba(0.28, 0.88, 1.0, 0.78),
        );

        let portrait = layout.portrait_rect();
        renderer.draw_rect(portrait, Color::rgba(0.015, 0.040, 0.052, 0.94));
        draw_border(
            renderer,
            portrait,
            1.0 * layout.scale,
            Color::rgba(0.33, 0.82, 0.95, 0.80),
        );
        renderer.draw_rect(
            inset_rect(portrait, 10.0 * layout.scale),
            Color::rgba(0.02, 0.07, 0.09, 0.72),
        );

        if let Some(image_size) = renderer.texture_size(EXPLORER_PORTRAIT_TEXTURE_ID) {
            renderer.draw_image(
                EXPLORER_PORTRAIT_TEXTURE_ID,
                contain_rect(inset_rect(portrait, 10.0 * layout.scale), image_size),
                Color::rgba(1.0, 1.0, 1.0, 1.0),
            );
        }

        let text_x = screen_x(viewport, layout.left_panel.origin.x + 28.0 * layout.scale);
        if let Some(callsign) = &self.text.callsign {
            draw_text(
                renderer,
                callsign,
                viewport,
                text_x,
                screen_y(viewport, layout.left_panel.origin.y + 406.0 * layout.scale),
                Color::rgba(0.92, 1.0, 0.98, 1.0),
            );
        }
        if let Some(role) = &self.text.role {
            draw_text(
                renderer,
                role,
                viewport,
                text_x,
                screen_y(viewport, layout.left_panel.origin.y + 440.0 * layout.scale),
                Color::rgba(0.58, 0.78, 0.84, 0.98),
            );
        }
        if let Some(id_line) = &self.text.id_line {
            draw_text(
                renderer,
                id_line,
                viewport,
                text_x,
                screen_y(viewport, layout.left_panel.origin.y + 466.0 * layout.scale),
                Color::rgba(0.78, 0.68, 0.48, 0.98),
            );
        }
    }

    fn draw_core(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &ProfileLayout) {
        draw_panel(
            renderer,
            layout.center_panel,
            Color::rgba(0.017, 0.027, 0.035, 0.94),
            Color::rgba(0.18, 0.29, 0.39, 0.78),
            layout.scale,
        );
        renderer.draw_rect(
            Rect::new(
                Vec2::new(
                    layout.center_panel.origin.x + 20.0 * layout.scale,
                    layout.center_panel.origin.y + 22.0 * layout.scale,
                ),
                Vec2::new(112.0 * layout.scale, 2.0 * layout.scale),
            ),
            Color::rgba(0.28, 0.88, 1.0, 0.78),
        );

        if let Some(header) = &self.text.status_header {
            draw_text(
                renderer,
                header,
                viewport,
                screen_x(viewport, layout.center_panel.origin.x + 26.0 * layout.scale),
                screen_y(viewport, layout.center_panel.origin.y + 42.0 * layout.scale),
                Color::rgba(0.78, 0.96, 1.0, 1.0),
            );
        }

        for (index, stat) in VITAL_STATS.iter().enumerate() {
            let y = layout.center_panel.origin.y + (86.0 + index as f32 * 38.0) * layout.scale;
            self.draw_percent_row(
                renderer,
                viewport,
                Vec2::new(layout.center_panel.origin.x + 28.0 * layout.scale, y),
                stat,
                &self.text.vital_labels,
                &self.text.vital_values,
                layout.scale,
            );
        }

        if let Some(header) = &self.text.core_header {
            draw_text(
                renderer,
                header,
                viewport,
                screen_x(viewport, layout.center_panel.origin.x + 26.0 * layout.scale),
                screen_y(
                    viewport,
                    layout.center_panel.origin.y + 244.0 * layout.scale,
                ),
                Color::rgba(0.78, 0.96, 1.0, 1.0),
            );
        }

        for (index, stat) in CORE_STATS.iter().enumerate() {
            let y = layout.center_panel.origin.y + (286.0 + index as f32 * 40.0) * layout.scale;
            self.draw_score_row(
                renderer,
                viewport,
                Vec2::new(layout.center_panel.origin.x + 28.0 * layout.scale, y),
                stat,
                layout.scale,
            );
        }
    }

    fn draw_research(&self, renderer: &mut dyn Renderer, viewport: Vec2, layout: &ProfileLayout) {
        draw_panel(
            renderer,
            layout.right_panel,
            Color::rgba(0.017, 0.027, 0.035, 0.94),
            Color::rgba(0.18, 0.29, 0.39, 0.78),
            layout.scale,
        );
        renderer.draw_rect(
            Rect::new(
                Vec2::new(
                    layout.right_panel.origin.x + 20.0 * layout.scale,
                    layout.right_panel.origin.y + 22.0 * layout.scale,
                ),
                Vec2::new(104.0 * layout.scale, 2.0 * layout.scale),
            ),
            Color::rgba(0.90, 0.70, 0.32, 0.88),
        );

        if let Some(header) = &self.text.research_header {
            draw_text(
                renderer,
                header,
                viewport,
                screen_x(viewport, layout.right_panel.origin.x + 26.0 * layout.scale),
                screen_y(viewport, layout.right_panel.origin.y + 42.0 * layout.scale),
                Color::rgba(0.78, 0.96, 1.0, 1.0),
            );
        }

        for (index, stat) in RESEARCH_STATS.iter().enumerate() {
            let y = layout.right_panel.origin.y + (90.0 + index as f32 * 46.0) * layout.scale;
            self.draw_percent_row(
                renderer,
                viewport,
                Vec2::new(layout.right_panel.origin.x + 28.0 * layout.scale, y),
                stat,
                &self.text.research_labels,
                &self.text.research_values,
                layout.scale,
            );
        }

        if let Some(header) = &self.text.resistance_header {
            draw_text(
                renderer,
                header,
                viewport,
                screen_x(viewport, layout.right_panel.origin.x + 26.0 * layout.scale),
                screen_y(viewport, layout.right_panel.origin.y + 270.0 * layout.scale),
                Color::rgba(0.78, 0.96, 1.0, 1.0),
            );
        }

        for (index, stat) in RESISTANCE_STATS.iter().enumerate() {
            let y = layout.right_panel.origin.y + (316.0 + index as f32 * 29.0) * layout.scale;
            self.draw_compact_percent_row(
                renderer,
                viewport,
                Vec2::new(layout.right_panel.origin.x + 28.0 * layout.scale, y),
                stat,
                layout.scale,
            );
        }
    }

    fn draw_percent_row(
        &self,
        renderer: &mut dyn Renderer,
        viewport: Vec2,
        origin: Vec2,
        stat: &PercentStat,
        labels: &HashMap<&'static str, TextSprite>,
        values: &HashMap<&'static str, TextSprite>,
        scale: f32,
    ) {
        if let Some(label) = labels.get(stat.id) {
            draw_text(
                renderer,
                label,
                viewport,
                screen_x(viewport, origin.x),
                screen_y(viewport, origin.y),
                Color::rgba(0.70, 0.88, 0.92, 0.96),
            );
        }

        if let Some(value) = values.get(stat.id) {
            draw_text(
                renderer,
                value,
                viewport,
                screen_x(viewport, origin.x + 196.0 * scale),
                screen_y(viewport, origin.y),
                Color::rgba(0.91, 1.0, 0.96, 1.0),
            );
        }

        self.draw_bar(
            renderer,
            Rect::new(
                Vec2::new(origin.x, origin.y + 22.0 * scale),
                Vec2::new(BAR_WIDTH * scale, BAR_HEIGHT * scale),
            ),
            stat.value as f32 / stat.max as f32,
            Color::rgba(0.33, 0.86, 1.0, 0.95),
            scale,
        );
    }

    fn draw_compact_percent_row(
        &self,
        renderer: &mut dyn Renderer,
        viewport: Vec2,
        origin: Vec2,
        stat: &PercentStat,
        scale: f32,
    ) {
        if let Some(label) = self.text.resistance_labels.get(stat.id) {
            draw_text(
                renderer,
                label,
                viewport,
                screen_x(viewport, origin.x),
                screen_y(viewport, origin.y),
                Color::rgba(0.70, 0.88, 0.92, 0.96),
            );
        }
        if let Some(value) = self.text.resistance_values.get(stat.id) {
            draw_text(
                renderer,
                value,
                viewport,
                screen_x(viewport, origin.x + 186.0 * scale),
                screen_y(viewport, origin.y),
                Color::rgba(0.91, 1.0, 0.96, 1.0),
            );
        }
    }

    fn draw_score_row(
        &self,
        renderer: &mut dyn Renderer,
        viewport: Vec2,
        origin: Vec2,
        stat: &ScoreStat,
        scale: f32,
    ) {
        if let Some(label) = self.text.core_labels.get(stat.id) {
            draw_text(
                renderer,
                label,
                viewport,
                screen_x(viewport, origin.x),
                screen_y(viewport, origin.y),
                Color::rgba(0.78, 0.93, 0.96, 0.98),
            );
        }

        if let Some(value) = self.text.core_values.get(stat.id) {
            draw_text(
                renderer,
                value,
                viewport,
                screen_x(viewport, origin.x + 238.0 * scale),
                screen_y(viewport, origin.y),
                Color::rgba(0.91, 1.0, 0.96, 1.0),
            );
        }

        let pip_y = origin.y + 30.0 * scale;
        for pip in 0..10 {
            let filled = pip < stat.value as usize;
            let rect = Rect::new(
                Vec2::new(
                    origin.x + pip as f32 * (SCORE_PIP_SIZE + SCORE_PIP_GAP) * scale,
                    pip_y,
                ),
                Vec2::new(SCORE_PIP_SIZE * scale, SCORE_PIP_SIZE * scale),
            );
            renderer.draw_rect(
                rect,
                if filled {
                    Color::rgba(0.25, 0.80, 0.92, 0.95)
                } else {
                    Color::rgba(0.035, 0.070, 0.085, 0.90)
                },
            );
            draw_border(
                renderer,
                rect,
                1.0 * scale,
                if filled {
                    Color::rgba(0.60, 0.96, 1.0, 0.92)
                } else {
                    Color::rgba(0.12, 0.22, 0.28, 0.80)
                },
            );
        }
    }

    fn draw_bar(
        &self,
        renderer: &mut dyn Renderer,
        rect: Rect,
        ratio: f32,
        fill: Color,
        scale: f32,
    ) {
        renderer.draw_rect(rect, Color::rgba(0.025, 0.050, 0.060, 0.95));
        renderer.draw_rect(
            Rect::new(
                rect.origin,
                Vec2::new(rect.size.x * ratio.clamp(0.0, 1.0), rect.size.y),
            ),
            fill,
        );
        draw_border(
            renderer,
            rect,
            1.0 * scale,
            Color::rgba(0.16, 0.31, 0.38, 0.82),
        );
    }

    fn upload_textures(&mut self, renderer: &mut dyn Renderer, font: &Font<'static>) -> Result<()> {
        let language = self.language;
        self.text = ProfileText {
            language: Some(language),
            ..ProfileText::default()
        };

        self.text.title = Some(upload_text(
            renderer,
            font,
            "profile_title",
            profile_title(language),
            match language {
                Language::Chinese => 34.0,
                Language::English => 30.0,
            },
        )?);
        self.text.callsign = Some(upload_text(
            renderer,
            font,
            "profile_callsign",
            callsign(language),
            match language {
                Language::Chinese => 30.0,
                Language::English => 27.0,
            },
        )?);
        self.text.role = Some(upload_text(
            renderer,
            font,
            "profile_role",
            role(language),
            20.0,
        )?);
        self.text.id_line = Some(upload_text(
            renderer,
            font,
            "profile_id_line",
            id_line(language),
            18.0,
        )?);
        self.text.status_header = Some(upload_text(
            renderer,
            font,
            "profile_status_header",
            status_header(language),
            21.0,
        )?);
        self.text.core_header = Some(upload_text(
            renderer,
            font,
            "profile_core_header",
            core_header(language),
            24.0,
        )?);
        self.text.research_header = Some(upload_text(
            renderer,
            font,
            "profile_research_header",
            research_header(language),
            23.0,
        )?);
        self.text.resistance_header = Some(upload_text(
            renderer,
            font,
            "profile_resistance_header",
            resistance_header(language),
            22.0,
        )?);

        for stat in VITAL_STATS {
            self.upload_percent_stat_text(
                renderer,
                font,
                stat,
                language,
                "profile_vital",
                ProfileTextTarget::Vital,
            )?;
        }
        for stat in CORE_STATS {
            self.text.core_labels.insert(
                stat.id,
                upload_text(
                    renderer,
                    font,
                    &format!("profile_core_label_{}", stat.id),
                    stat.label.get(language),
                    21.0,
                )?,
            );
            self.text.core_values.insert(
                stat.id,
                upload_text(
                    renderer,
                    font,
                    &format!("profile_core_value_{}", stat.id),
                    &format!("{}/10", stat.value),
                    18.0,
                )?,
            );
        }
        for stat in RESEARCH_STATS {
            self.upload_percent_stat_text(
                renderer,
                font,
                stat,
                language,
                "profile_research",
                ProfileTextTarget::Research,
            )?;
        }
        for stat in RESISTANCE_STATS {
            self.upload_percent_stat_text(
                renderer,
                font,
                stat,
                language,
                "profile_resistance",
                ProfileTextTarget::Resistance,
            )?;
        }

        Ok(())
    }

    fn upload_percent_stat_text(
        &mut self,
        renderer: &mut dyn Renderer,
        font: &Font<'static>,
        stat: &PercentStat,
        language: Language,
        prefix: &str,
        target: ProfileTextTarget,
    ) -> Result<()> {
        let label = upload_text(
            renderer,
            font,
            &format!("{prefix}_label_{}", stat.id),
            stat.label.get(language),
            18.0,
        )?;
        let value = upload_text(
            renderer,
            font,
            &format!("{prefix}_value_{}", stat.id),
            &format!("{}/{}", stat.value, stat.max),
            16.0,
        )?;

        match target {
            ProfileTextTarget::Vital => {
                self.text.vital_labels.insert(stat.id, label);
                self.text.vital_values.insert(stat.id, value);
            }
            ProfileTextTarget::Research => {
                self.text.research_labels.insert(stat.id, label);
                self.text.research_values.insert(stat.id, value);
            }
            ProfileTextTarget::Resistance => {
                self.text.resistance_labels.insert(stat.id, label);
                self.text.resistance_values.insert(stat.id, value);
            }
        }

        Ok(())
    }
}

enum ProfileTextTarget {
    Vital,
    Research,
    Resistance,
}

impl Scene for ProfileScene {
    fn id(&self) -> SceneId {
        SceneId::Profile
    }

    fn name(&self) -> &str {
        "ProfileScene"
    }

    fn setup(&mut self, renderer: &mut dyn Renderer) -> Result<()> {
        if renderer
            .texture_size(EXPLORER_PORTRAIT_TEXTURE_ID)
            .is_none()
        {
            renderer.load_texture(
                EXPLORER_PORTRAIT_TEXTURE_ID,
                Path::new(EXPLORER_PORTRAIT_PATH),
            )?;
        }

        let font = load_ui_font()?;
        self.upload_textures(renderer, &font)
    }

    fn update(
        &mut self,
        ctx: &mut GameContext,
        _dt: f32,
        input: &InputState,
    ) -> Result<SceneCommand<SceneId>> {
        if self.language != ctx.language {
            self.language = ctx.language;
            self.text = ProfileText::default();
        }

        if input.just_pressed(Button::Pause) || input.just_pressed(Button::Profile) {
            return Ok(SceneCommand::Pop);
        }

        Ok(SceneCommand::None)
    }

    fn render(&mut self, ctx: &mut RenderContext<'_>) -> Result<()> {
        if self.text.language != Some(self.language) {
            let font = load_ui_font()?;
            self.upload_textures(ctx.renderer, &font)?;
        }

        self.draw_profile(ctx);
        Ok(())
    }
}

struct ProfileLayout {
    scale: f32,
    root_panel: Rect,
    header: Rect,
    left_panel: Rect,
    center_panel: Rect,
    right_panel: Rect,
}

impl ProfileLayout {
    fn new(viewport: Vec2) -> Self {
        let scale = ((viewport.x - 48.0) / PROFILE_WIDTH)
            .min((viewport.y - 48.0) / PROFILE_HEIGHT)
            .min(1.0)
            .max(0.64);
        let root_panel = Rect::new(
            Vec2::new(-PROFILE_WIDTH * scale * 0.5, -PROFILE_HEIGHT * scale * 0.5),
            Vec2::new(PROFILE_WIDTH * scale, PROFILE_HEIGHT * scale),
        );
        let header = Rect::new(
            root_panel.origin,
            Vec2::new(root_panel.size.x, HEADER_HEIGHT * scale),
        );
        let content_y = root_panel.origin.y + (HEADER_HEIGHT + 16.0) * scale;
        let left_origin = Vec2::new(root_panel.origin.x + OUTER_PADDING * scale, content_y);
        let center_origin = Vec2::new(
            left_origin.x + (LEFT_PANEL_SIZE.x + CONTENT_GAP) * scale,
            content_y,
        );
        let right_origin = Vec2::new(
            center_origin.x + (CENTER_PANEL_SIZE.x + CONTENT_GAP) * scale,
            content_y,
        );

        Self {
            scale,
            root_panel,
            header,
            left_panel: Rect::new(left_origin, LEFT_PANEL_SIZE * scale),
            center_panel: Rect::new(center_origin, CENTER_PANEL_SIZE * scale),
            right_panel: Rect::new(right_origin, RIGHT_PANEL_SIZE * scale),
        }
    }

    fn portrait_rect(&self) -> Rect {
        Rect::new(
            Vec2::new(
                self.left_panel.origin.x + 85.0 * self.scale,
                self.left_panel.origin.y + 42.0 * self.scale,
            ),
            Vec2::new(250.0 * self.scale, 360.0 * self.scale),
        )
    }
}

fn draw_panel(renderer: &mut dyn Renderer, rect: Rect, fill: Color, border: Color, scale: f32) {
    renderer.draw_rect(rect, fill);
    draw_border(renderer, rect, 1.0 * scale, border);
    draw_border(
        renderer,
        inset_rect(rect, 4.0 * scale),
        1.0 * scale,
        Color::rgba(0.08, 0.16, 0.21, 0.60),
    );
}

fn draw_border(renderer: &mut dyn Renderer, rect: Rect, thickness: f32, color: Color) {
    let thickness = thickness.max(1.0);
    renderer.draw_rect(
        Rect::new(rect.origin, Vec2::new(rect.size.x, thickness)),
        color,
    );
    renderer.draw_rect(
        Rect::new(
            Vec2::new(rect.origin.x, rect.bottom() - thickness),
            Vec2::new(rect.size.x, thickness),
        ),
        color,
    );
    renderer.draw_rect(
        Rect::new(rect.origin, Vec2::new(thickness, rect.size.y)),
        color,
    );
    renderer.draw_rect(
        Rect::new(
            Vec2::new(rect.right() - thickness, rect.origin.y),
            Vec2::new(thickness, rect.size.y),
        ),
        color,
    );
}

fn draw_corner_brackets(
    renderer: &mut dyn Renderer,
    rect: Rect,
    length: f32,
    thickness: f32,
    color: Color,
) {
    let thickness = thickness.max(1.0);
    for (x, y, horizontal_x, vertical_y) in [
        (rect.origin.x, rect.origin.y, rect.origin.x, rect.origin.y),
        (
            rect.right() - length,
            rect.origin.y,
            rect.right() - thickness,
            rect.origin.y,
        ),
        (
            rect.origin.x,
            rect.bottom() - thickness,
            rect.origin.x,
            rect.bottom() - length,
        ),
        (
            rect.right() - length,
            rect.bottom() - thickness,
            rect.right() - thickness,
            rect.bottom() - length,
        ),
    ] {
        renderer.draw_rect(
            Rect::new(Vec2::new(x, y), Vec2::new(length, thickness)),
            color,
        );
        renderer.draw_rect(
            Rect::new(
                Vec2::new(horizontal_x, vertical_y),
                Vec2::new(thickness, length),
            ),
            color,
        );
    }
}

fn inset_rect(rect: Rect, inset: f32) -> Rect {
    Rect::new(
        Vec2::new(rect.origin.x + inset, rect.origin.y + inset),
        Vec2::new(
            (rect.size.x - inset * 2.0).max(0.0),
            (rect.size.y - inset * 2.0).max(0.0),
        ),
    )
}

fn contain_rect(frame: Rect, image_size: Vec2) -> Rect {
    if image_size.x <= 0.0 || image_size.y <= 0.0 || frame.size.x <= 0.0 || frame.size.y <= 0.0 {
        return frame;
    }

    let scale = (frame.size.x / image_size.x).min(frame.size.y / image_size.y);
    let size = image_size * scale;
    Rect::new(
        Vec2::new(
            frame.origin.x + (frame.size.x - size.x) * 0.5,
            frame.origin.y + (frame.size.y - size.y) * 0.5,
        ),
        size,
    )
}

pub(super) fn profile_overview(language: Language) -> ProfileOverview {
    ProfileOverview {
        callsign: callsign(language),
        role: role(language),
        id_line: id_line(language),
        vital_stats: VITAL_STATS
            .iter()
            .map(|stat| ProfilePercentView {
                label: stat.label.get(language),
                value: stat.value,
                max: stat.max,
            })
            .collect(),
        core_stats: CORE_STATS
            .iter()
            .map(|stat| ProfileScoreView {
                label: stat.label.get(language),
                value: stat.value,
            })
            .collect(),
        research_stats: RESEARCH_STATS
            .iter()
            .map(|stat| ProfilePercentView {
                label: stat.label.get(language),
                value: stat.value,
                max: stat.max,
            })
            .collect(),
    }
}

fn profile_title(language: Language) -> &'static str {
    match language {
        Language::Chinese => "外勤档案",
        Language::English => "FIELD DOSSIER",
    }
}

fn callsign(language: Language) -> &'static str {
    match language {
        Language::Chinese => "星尘调查员",
        Language::English => "Stardust Surveyor",
    }
}

fn role(language: Language) -> &'static str {
    match language {
        Language::Chinese => "先遣探索员 / 样本分析",
        Language::English => "Forward Explorer / Sample Analysis",
    }
}

fn id_line(language: Language) -> &'static str {
    match language {
        Language::Chinese => "外勤编号: AA-01",
        Language::English => "Field ID: AA-01",
    }
}

fn status_header(language: Language) -> &'static str {
    match language {
        Language::Chinese => "生命状态",
        Language::English => "Vital Status",
    }
}

fn core_header(language: Language) -> &'static str {
    match language {
        Language::Chinese => "探索能力",
        Language::English => "Explorer Aptitudes",
    }
}

fn research_header(language: Language) -> &'static str {
    match language {
        Language::Chinese => "研究专精",
        Language::English => "Research Focus",
    }
}

fn resistance_header(language: Language) -> &'static str {
    match language {
        Language::Chinese => "环境抗性",
        Language::English => "Environmental Resistances",
    }
}

fn screen_rect(viewport: Vec2, x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(
        Vec2::new(-viewport.x * 0.5 + x, -viewport.y * 0.5 + y),
        Vec2::new(width, height),
    )
}

fn screen_x(viewport: Vec2, world_x: f32) -> f32 {
    world_x + viewport.x * 0.5
}

fn screen_y(viewport: Vec2, world_y: f32) -> f32 {
    world_y + viewport.y * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_has_balanced_mvp_stat_groups() {
        assert_eq!(VITAL_STATS.len(), 4);
        assert_eq!(CORE_STATS.len(), 5);
        assert_eq!(RESEARCH_STATS.len(), 4);
        assert_eq!(RESISTANCE_STATS.len(), 4);

        for stat in VITAL_STATS
            .iter()
            .chain(RESEARCH_STATS.iter())
            .chain(RESISTANCE_STATS.iter())
        {
            assert!(stat.value <= stat.max, "{} should not exceed max", stat.id);
        }

        for stat in CORE_STATS {
            assert!(stat.value <= 10, "{} should be a 0-10 score", stat.id);
        }
    }

    #[test]
    fn profile_text_has_chinese_and_english_strings() {
        for language in Language::SUPPORTED {
            assert!(!profile_title(language).is_empty());
            assert!(!callsign(language).is_empty());
            assert!(!role(language).is_empty());
            assert!(!id_line(language).is_empty());
            assert!(!status_header(language).is_empty());
            assert!(!core_header(language).is_empty());
            assert!(!research_header(language).is_empty());
            assert!(!resistance_header(language).is_empty());

            for stat in VITAL_STATS
                .iter()
                .chain(RESEARCH_STATS.iter())
                .chain(RESISTANCE_STATS.iter())
            {
                assert!(!stat.label.get(language).is_empty());
            }

            for stat in CORE_STATS {
                assert!(!stat.label.get(language).is_empty());
            }
        }
    }

    #[test]
    fn profile_generated_portrait_path_exists() {
        let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert!(project_root.join(EXPLORER_PORTRAIT_PATH).exists());
    }

    #[test]
    fn contain_rect_preserves_image_aspect_ratio() {
        let frame = Rect::new(Vec2::ZERO, Vec2::new(250.0, 360.0));
        let contained = contain_rect(frame, Vec2::new(1024.0, 1536.0));

        assert!((contained.size.x / contained.size.y - 1024.0 / 1536.0).abs() < 0.001);
        assert!(contained.size.x <= frame.size.x);
        assert!(contained.size.y <= frame.size.y);
    }
}
