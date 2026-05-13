use anyhow::{Result, bail};
use content::{CutsceneCompletion, CutsceneDefinition, CutsceneStep, CutsceneText};
use runtime::{Button, Color, InputState, Rect, Renderer, SceneCommand, Vec2};
use rusttype::Font;

use crate::ui::{
    menu_widgets::{draw_border, draw_screen_rect},
    text::{TextSprite, draw_text, draw_text_centered, load_ui_font, upload_text},
};

use super::{GameContext, Language, RenderContext, Scene, SceneId};

const PANEL_WIDTH: f32 = 760.0;
const PANEL_MIN_HEIGHT: f32 = 210.0;
const PANEL_PADDING_X: f32 = 34.0;
const PANEL_PADDING_Y: f32 = 28.0;
const BODY_LINE_GAP: f32 = 6.0;
const BODY_WRAP_CHARS: usize = 46;

#[derive(Default)]
struct CutsceneTextSprites {
    panels: Vec<Option<CutscenePanelText>>,
    continue_prompt: Option<TextSprite>,
    skip_prompt: Option<TextSprite>,
}

struct CutscenePanelText {
    speaker: Option<TextSprite>,
    body_lines: Vec<TextSprite>,
}

pub(super) struct CutsceneScene {
    definition: CutsceneDefinition,
    language: Language,
    step_index: usize,
    step_elapsed: f32,
    pending_cleared: bool,
    text: CutsceneTextSprites,
}

impl CutsceneScene {
    pub(super) fn new(ctx: &GameContext) -> Result<Self> {
        let Some(definition) = ctx.pending_cutscene_definition() else {
            bail!("CutsceneScene requested without a pending cutscene");
        };

        Ok(Self {
            definition,
            language: ctx.language,
            step_index: 0,
            step_elapsed: 0.0,
            pending_cleared: false,
            text: CutsceneTextSprites::default(),
        })
    }

    fn upload_textures(&mut self, renderer: &mut dyn Renderer, font: &Font<'static>) -> Result<()> {
        self.text.panels = self
            .definition
            .steps
            .iter()
            .enumerate()
            .map(|(index, step)| {
                let CutsceneStep::TextPanel { speaker, body, .. } = step else {
                    return Ok(None);
                };

                let speaker = speaker
                    .as_ref()
                    .map(|speaker| {
                        upload_text(
                            renderer,
                            font,
                            &format!("cutscene_speaker_{index}"),
                            localized_cutscene_text(speaker, self.language),
                            18.0,
                        )
                    })
                    .transpose()?;
                let body_lines = wrap_cutscene_text(localized_cutscene_text(body, self.language))
                    .iter()
                    .enumerate()
                    .map(|(line_index, line)| {
                        upload_text(
                            renderer,
                            font,
                            &format!("cutscene_body_{index}_{line_index}"),
                            line,
                            25.0,
                        )
                    })
                    .collect::<Result<Vec<_>>>()?;

                Ok(Some(CutscenePanelText {
                    speaker,
                    body_lines,
                }))
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.continue_prompt = Some(upload_text(
            renderer,
            font,
            "cutscene_continue_prompt",
            continue_prompt(self.language),
            15.0,
        )?);
        self.text.skip_prompt = Some(upload_text(
            renderer,
            font,
            "cutscene_skip_prompt",
            skip_prompt(self.language),
            14.0,
        )?);
        Ok(())
    }

    fn current_step(&self) -> Option<&CutsceneStep> {
        self.definition.steps.get(self.step_index)
    }

    fn advance_step(&mut self) {
        self.step_index += 1;
        self.step_elapsed = 0.0;
    }

    fn finish(&mut self, ctx: &mut GameContext) -> SceneCommand<SceneId> {
        ctx.mark_cutscene_seen(&self.definition.id);
        match &self.definition.completion {
            CutsceneCompletion::Pop => SceneCommand::Pop,
            CutsceneCompletion::SwitchScene { scene } => scene_id_from_cutscene_key(scene)
                .map(SceneCommand::Switch)
                .unwrap_or(SceneCommand::Pop),
        }
    }

    fn draw_fade(&self, renderer: &mut dyn Renderer, viewport: Vec2, alpha: f32) {
        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(Vec2::ZERO, viewport),
            Color::rgba(0.0, 0.0, 0.0, alpha.clamp(0.0, 1.0)),
        );
    }

    fn draw_full_black(&self, renderer: &mut dyn Renderer, viewport: Vec2) {
        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(Vec2::ZERO, viewport),
            Color::rgba(0.002, 0.006, 0.010, 0.98),
        );
    }

    fn draw_text_panel(&self, renderer: &mut dyn Renderer, viewport: Vec2, step_index: usize) {
        self.draw_full_black(renderer, viewport);
        let Some(Some(panel_text)) = self.text.panels.get(step_index) else {
            return;
        };

        let body_height = panel_text
            .body_lines
            .iter()
            .map(|line| line.size.y + BODY_LINE_GAP)
            .sum::<f32>();
        let speaker_height = panel_text
            .speaker
            .as_ref()
            .map(|speaker| speaker.size.y + 12.0)
            .unwrap_or(0.0);
        let panel_height =
            (PANEL_PADDING_Y * 2.0 + speaker_height + body_height + 38.0).max(PANEL_MIN_HEIGHT);
        let panel_width = PANEL_WIDTH.min(viewport.x - 56.0).max(280.0);
        let panel = Rect::new(
            Vec2::new(
                (viewport.x - panel_width) * 0.5,
                viewport.y - panel_height - 82.0,
            ),
            Vec2::new(panel_width, panel_height),
        );

        draw_screen_rect(
            renderer,
            viewport,
            panel,
            Color::rgba(0.018, 0.044, 0.056, 0.94),
        );
        draw_border(
            renderer,
            viewport,
            panel,
            1.0,
            Color::rgba(0.34, 0.86, 1.0, 0.60),
        );

        let mut y = panel.origin.y + PANEL_PADDING_Y;
        if let Some(speaker) = &panel_text.speaker {
            draw_text(
                renderer,
                speaker,
                viewport,
                panel.origin.x + PANEL_PADDING_X,
                y,
                Color::rgba(0.52, 0.94, 1.0, 1.0),
            );
            y += speaker.size.y + 12.0;
        }

        for line in &panel_text.body_lines {
            draw_text(
                renderer,
                line,
                viewport,
                panel.origin.x + PANEL_PADDING_X,
                y,
                Color::rgba(0.88, 1.0, 0.96, 1.0),
            );
            y += line.size.y + BODY_LINE_GAP;
        }

        if let Some(prompt) = &self.text.continue_prompt {
            draw_text(
                renderer,
                prompt,
                viewport,
                panel.right() - prompt.size.x - PANEL_PADDING_X,
                panel.bottom() - prompt.size.y - 16.0,
                Color::rgba(0.62, 0.86, 0.92, 0.88),
            );
        }
        if let Some(skip) = &self.text.skip_prompt {
            draw_text_centered(
                renderer,
                skip,
                viewport,
                viewport.x * 0.5,
                viewport.y - 34.0,
                Color::rgba(0.50, 0.66, 0.72, 0.70),
            );
        }
    }
}

impl Scene for CutsceneScene {
    fn id(&self) -> SceneId {
        SceneId::Cutscene
    }

    fn name(&self) -> &str {
        "CutsceneScene"
    }

    fn setup(&mut self, renderer: &mut dyn Renderer) -> Result<()> {
        let font = load_ui_font()?;
        self.upload_textures(renderer, &font)
    }

    fn update(
        &mut self,
        ctx: &mut GameContext,
        dt: f32,
        input: &InputState,
    ) -> Result<SceneCommand<SceneId>> {
        if !self.pending_cleared {
            ctx.clear_pending_cutscene(&self.definition.id);
            self.pending_cleared = true;
        }

        if input.just_pressed(Button::Pause) {
            self.step_index = self.definition.steps.len();
            return Ok(self.finish(ctx));
        }

        self.step_elapsed += dt;
        let advance_requested = input.just_pressed(Button::Confirm)
            || input.just_pressed(Button::Interact)
            || input.mouse_left_just_pressed();

        loop {
            let Some(step) = self.current_step() else {
                return Ok(self.finish(ctx));
            };

            match step {
                CutsceneStep::FadeIn { duration }
                | CutsceneStep::FadeOut { duration }
                | CutsceneStep::Wait { duration } => {
                    if self.step_elapsed >= duration.max(0.0) {
                        self.advance_step();
                        continue;
                    }
                }
                CutsceneStep::TextPanel {
                    min_duration,
                    require_confirm,
                    ..
                } => {
                    let ready = self.step_elapsed >= min_duration.max(0.0);
                    let should_advance = if *require_confirm {
                        ready && advance_requested
                    } else {
                        ready
                    };
                    if should_advance {
                        self.advance_step();
                        continue;
                    }
                }
                CutsceneStep::SetFlag { flag } => {
                    ctx.mark_cutscene_flag(flag);
                    self.advance_step();
                    continue;
                }
            }

            break;
        }

        Ok(SceneCommand::None)
    }

    fn render(&mut self, ctx: &mut RenderContext<'_>) -> Result<()> {
        let viewport = ctx.renderer.screen_size();
        match self.current_step() {
            Some(CutsceneStep::FadeIn { duration }) => {
                let alpha = 1.0 - self.step_elapsed / duration.max(0.001);
                self.draw_fade(ctx.renderer, viewport, alpha);
            }
            Some(CutsceneStep::FadeOut { duration }) => {
                let alpha = self.step_elapsed / duration.max(0.001);
                self.draw_fade(ctx.renderer, viewport, alpha);
            }
            Some(CutsceneStep::TextPanel { .. }) => {
                self.draw_text_panel(ctx.renderer, viewport, self.step_index);
            }
            Some(CutsceneStep::Wait { .. }) | Some(CutsceneStep::SetFlag { .. }) | None => {
                self.draw_full_black(ctx.renderer, viewport);
            }
        }

        Ok(())
    }
}

fn localized_cutscene_text(text: &CutsceneText, language: Language) -> &str {
    match language {
        Language::Chinese if !text.chinese.trim().is_empty() => &text.chinese,
        Language::English if !text.english.trim().is_empty() => &text.english,
        _ if !text.english.trim().is_empty() => &text.english,
        _ => &text.chinese,
    }
}

fn continue_prompt(language: Language) -> &'static str {
    match language {
        Language::Chinese => "Enter / Space 继续",
        Language::English => "Enter / Space Continue",
    }
}

fn skip_prompt(language: Language) -> &'static str {
    match language {
        Language::Chinese => "Esc 跳过过场",
        Language::English => "Esc Skip Cutscene",
    }
}

fn wrap_cutscene_text(text: &str) -> Vec<String> {
    text.lines()
        .flat_map(|line| wrap_cutscene_line(line.trim(), BODY_WRAP_CHARS))
        .collect::<Vec<_>>()
        .into_iter()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
}

fn wrap_cutscene_line(line: &str, max_chars: usize) -> Vec<String> {
    if line.is_empty() {
        return Vec::new();
    }

    if line.contains(char::is_whitespace) {
        return wrap_words(line, max_chars);
    }

    let chars = line.chars().collect::<Vec<_>>();
    chars
        .chunks(max_chars.max(1))
        .map(|chunk| chunk.iter().collect())
        .collect()
}

fn wrap_words(line: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in line.split_whitespace() {
        let next_len = if current.is_empty() {
            word.chars().count()
        } else {
            current.chars().count() + 1 + word.chars().count()
        };
        if next_len > max_chars && !current.is_empty() {
            lines.push(current);
            current = String::new();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn scene_id_from_cutscene_key(value: &str) -> Option<SceneId> {
    match value.trim().to_ascii_lowercase().as_str() {
        "mainmenu" | "main_menu" | "main-menu" => Some(SceneId::MainMenu),
        "overworld" => Some(SceneId::Overworld),
        "facility" => Some(SceneId::Facility),
        "game_menu" | "gamemenu" | "game-menu" => Some(SceneId::GameMenu),
        "inventory" => Some(SceneId::Inventory),
        "profile" => Some(SceneId::Profile),
        "pause" => Some(SceneId::Pause),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_cutscene_text_splits_long_english_lines() {
        let lines =
            wrap_cutscene_text("alpha beta gamma delta epsilon zeta eta theta iota kappa lambda");

        assert!(lines.len() > 1);
        assert!(
            lines
                .iter()
                .all(|line| line.chars().count() <= BODY_WRAP_CHARS)
        );
    }

    #[test]
    fn wrap_cutscene_text_splits_cjk_without_spaces() {
        let lines = wrap_cutscene_text(
            "这是一个很长很长的中文过场文本用于验证没有空格时也能自动换行避免文字溢出面板",
        );

        assert!(!lines.is_empty());
        assert!(
            lines
                .iter()
                .all(|line| line.chars().count() <= BODY_WRAP_CHARS)
        );
    }

    #[test]
    fn scene_keys_map_to_scene_ids() {
        assert_eq!(
            scene_id_from_cutscene_key("Overworld"),
            Some(SceneId::Overworld)
        );
        assert_eq!(
            scene_id_from_cutscene_key("main_menu"),
            Some(SceneId::MainMenu)
        );
        assert_eq!(scene_id_from_cutscene_key("unknown"), None);
    }
}
