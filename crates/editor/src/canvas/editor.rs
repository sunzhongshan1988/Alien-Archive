use crate::*;

#[derive(Clone, Copy, Debug)]
struct SelectionMapBounds {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl SelectionMapBounds {
    fn from_min_max(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    fn include(&mut self, other: Self) {
        self.min_x = self.min_x.min(other.min_x);
        self.min_y = self.min_y.min(other.min_y);
        self.max_x = self.max_x.max(other.max_x);
        self.max_y = self.max_y.max(other.max_y);
    }

    fn center_x(self) -> f32 {
        (self.min_x + self.max_x) * 0.5
    }

    fn center_y(self) -> f32 {
        (self.min_y + self.max_y) * 0.5
    }

    fn width(self) -> f32 {
        self.max_x - self.min_x
    }

    fn height(self) -> f32 {
        self.max_y - self.min_y
    }
}

fn align_target(mode: BatchAlignMode, bounds: SelectionMapBounds) -> f32 {
    match mode {
        BatchAlignMode::Left => bounds.min_x,
        BatchAlignMode::CenterX => bounds.center_x(),
        BatchAlignMode::Right => bounds.max_x,
        BatchAlignMode::Top => bounds.min_y,
        BatchAlignMode::CenterY => bounds.center_y(),
        BatchAlignMode::Bottom => bounds.max_y,
    }
}

fn align_delta(mode: BatchAlignMode, bounds: SelectionMapBounds, target: f32) -> [f32; 2] {
    match mode {
        BatchAlignMode::Left => [target - bounds.min_x, 0.0],
        BatchAlignMode::CenterX => [target - bounds.center_x(), 0.0],
        BatchAlignMode::Right => [target - bounds.max_x, 0.0],
        BatchAlignMode::Top => [0.0, target - bounds.min_y],
        BatchAlignMode::CenterY => [0.0, target - bounds.center_y()],
        BatchAlignMode::Bottom => [0.0, target - bounds.max_y],
    }
}

fn zone_map_bounds(points: &[[f32; 2]]) -> Option<SelectionMapBounds> {
    let first = points.first()?;
    let mut bounds = SelectionMapBounds::from_min_max(first[0], first[1], first[0], first[1]);
    for point in points.iter().skip(1) {
        bounds.include(SelectionMapBounds::from_min_max(
            point[0], point[1], point[0], point[1],
        ));
    }
    Some(bounds)
}

fn is_meaningful_delta(delta: [f32; 2]) -> bool {
    delta[0].abs() > 0.001 || delta[1].abs() > 0.001
}

fn trimmed_optional(value: &str) -> Option<String> {
    let value = value.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

fn normalized_optional(value: Option<&String>) -> Option<String> {
    value.and_then(|value| trimmed_optional(value))
}

fn common_text_value<I>(values: I) -> (String, bool)
where
    I: IntoIterator<Item = String>,
{
    let mut values = values.into_iter();
    let Some(first) = values.next() else {
        return (String::new(), false);
    };
    if values.any(|value| value != first) {
        (String::new(), true)
    } else {
        (first, false)
    }
}

fn common_optional_text_value<I>(values: I) -> (String, bool)
where
    I: IntoIterator<Item = Option<String>>,
{
    let mut values = values.into_iter();
    let Some(first) = values.next() else {
        return (String::new(), false);
    };
    if values.any(|value| value != first) {
        (String::new(), true)
    } else {
        (first.unwrap_or_default(), false)
    }
}

fn distribute_deltas(
    mode: BatchDistributeMode,
    mut candidates: Vec<(usize, SelectionMapBounds)>,
) -> Vec<(usize, [f32; 2])> {
    if candidates.len() < 3 {
        return Vec::new();
    }

    match mode {
        BatchDistributeMode::Horizontal => {
            candidates.sort_by(|left, right| left.1.min_x.total_cmp(&right.1.min_x));
            let min = candidates
                .iter()
                .map(|(_, bounds)| bounds.min_x)
                .fold(f32::INFINITY, f32::min);
            let max = candidates
                .iter()
                .map(|(_, bounds)| bounds.max_x)
                .fold(f32::NEG_INFINITY, f32::max);
            let total_size = candidates
                .iter()
                .map(|(_, bounds)| bounds.width().max(0.0))
                .sum::<f32>();
            let gap = (max - min - total_size) / (candidates.len() - 1) as f32;
            let mut cursor = min;
            candidates
                .into_iter()
                .filter_map(|(index, bounds)| {
                    let delta = [cursor - bounds.min_x, 0.0];
                    cursor += bounds.width().max(0.0) + gap;
                    is_meaningful_delta(delta).then_some((index, delta))
                })
                .collect()
        }
        BatchDistributeMode::Vertical => {
            candidates.sort_by(|left, right| left.1.min_y.total_cmp(&right.1.min_y));
            let min = candidates
                .iter()
                .map(|(_, bounds)| bounds.min_y)
                .fold(f32::INFINITY, f32::min);
            let max = candidates
                .iter()
                .map(|(_, bounds)| bounds.max_y)
                .fold(f32::NEG_INFINITY, f32::max);
            let total_size = candidates
                .iter()
                .map(|(_, bounds)| bounds.height().max(0.0))
                .sum::<f32>();
            let gap = (max - min - total_size) / (candidates.len() - 1) as f32;
            let mut cursor = min;
            candidates
                .into_iter()
                .filter_map(|(index, bounds)| {
                    let delta = [0.0, cursor - bounds.min_y];
                    cursor += bounds.height().max(0.0) + gap;
                    is_meaningful_delta(delta).then_some((index, delta))
                })
                .collect()
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BatchUnlockField {
    Codex,
    Item,
    Message,
}

static EMPTY_UNLOCK_RULE: UnlockRule = UnlockRule {
    requires_codex_id: None,
    requires_item_id: None,
    locked_message: None,
};

impl EditorApp {
    pub(crate) fn draw_canvas(&mut self, ui: &mut egui::Ui, ctx: &EguiContext) {
        let desired_size = ui.available_size_before_wrap();
        let (response, painter) = ui.allocate_painter(desired_size, Sense::click_and_drag());
        let rect = response.rect;
        self.last_canvas_rect = Some(rect);

        if let Some(focus_world) = self.pending_focus_world.take() {
            self.pan = (rect.center() - rect.min) - focus_world * self.zoom;
        }

        painter.rect_filled(rect, 0.0, THEME_CANVAS_BG);
        self.apply_canvas_input(&response, ctx);

        let tile_size = self.document.tile_size as f32;
        let map_size = vec2(
            self.document.width as f32 * tile_size,
            self.document.height as f32 * tile_size,
        );
        let map_rect = Rect::from_min_size(
            self.world_to_screen(rect, vec2(0.0, 0.0)),
            map_size * self.zoom,
        );
        painter.rect_filled(map_rect, 0.0, THEME_MAP_BG);

        let visible_asset_ids = self.visible_canvas_asset_ids(rect);
        let requested_textures =
            self.request_asset_textures(visible_asset_ids.iter().map(String::as_str));
        let loaded_visible_textures = visible_asset_ids
            .iter()
            .filter(|asset_id| self.thumbnails.contains_key(asset_id.as_str()))
            .count();
        let map_textures_ready = loaded_visible_textures >= visible_asset_ids.len();
        if requested_textures > 0 || !map_textures_ready {
            ctx.request_repaint();
        }

        if !map_textures_ready {
            if self.show_grid {
                draw_grid(
                    &painter,
                    rect,
                    map_rect,
                    self.document.width,
                    self.document.height,
                    tile_size * self.zoom,
                );
            }
            self.draw_map_texture_loading_overlay(
                &painter,
                rect,
                loaded_visible_textures,
                visible_asset_ids.len(),
            );
            self.mouse_tile = None;
            return;
        }

        self.draw_layers(rect, &painter);

        if self.show_grid {
            draw_grid(
                &painter,
                rect,
                map_rect,
                self.document.width,
                self.document.height,
                tile_size * self.zoom,
            );
        }

        if self.show_collision {
            self.draw_collision(rect, &painter);
        }

        if self.show_entity_bounds {
            self.draw_entity_bounds(rect, &painter);
        }

        self.draw_selection_bounds(rect, &painter);
        self.handle_canvas_context_menu(&response, rect, ctx);

        if response.hovered() {
            if let Some(mouse) = response.hover_pos() {
                self.mouse_tile = self.screen_to_tile(rect, mouse);
            }
        } else {
            self.mouse_tile = None;
        }

        self.draw_brush_preview(
            rect,
            &painter,
            response.hover_pos(),
            ctx.input(|input| input.modifiers),
        );
        self.draw_stamp_preview(rect, &painter, response.hover_pos());
        self.draw_rectangle_preview(rect, &painter);
        self.draw_zone_draft(
            rect,
            &painter,
            response.hover_pos(),
            ctx.input(|input| input.modifiers),
        );
        self.draw_selection_marquee(&painter);
        self.handle_canvas_selection(&response, rect, ctx);
        self.handle_canvas_placement(&response, rect, ctx);
    }

    fn visible_canvas_asset_ids(&self, canvas_rect: Rect) -> Vec<String> {
        let visible_tiles = self.visible_tile_bounds(canvas_rect);
        let mut asset_ids = BTreeSet::new();

        if self.layer_state(LayerKind::Ground).visible {
            for tile in &self.document.layers.ground {
                if tile_intersects_rect(
                    tile,
                    visible_tiles.min_x,
                    visible_tiles.min_y,
                    visible_tiles.max_x,
                    visible_tiles.max_y,
                ) && self.registry.get(&tile.asset).is_some()
                {
                    asset_ids.insert(tile.asset.clone());
                }
            }
        }

        if self.layer_state(LayerKind::Decals).visible {
            for decal in &self.document.layers.decals {
                let Some(rect) = self.object_instance_screen_rect(canvas_rect, decal) else {
                    continue;
                };
                if rect.intersects(canvas_rect) && self.registry.get(&decal.asset).is_some() {
                    asset_ids.insert(decal.asset.clone());
                }
            }
        }

        if self.layer_state(LayerKind::Objects).visible {
            for object in &self.document.layers.objects {
                let Some(rect) = self.object_instance_screen_rect(canvas_rect, object) else {
                    continue;
                };
                if rect.intersects(canvas_rect) && self.registry.get(&object.asset).is_some() {
                    asset_ids.insert(object.asset.clone());
                }
            }
        }

        if self.layer_state(LayerKind::Entities).visible {
            for entity in &self.document.layers.entities {
                let Some(rect) = self.entity_instance_screen_rect(canvas_rect, entity) else {
                    continue;
                };
                if rect.intersects(canvas_rect) && self.registry.get(&entity.asset).is_some() {
                    asset_ids.insert(entity.asset.clone());
                }
            }
        }

        asset_ids.into_iter().collect()
    }

    fn draw_map_texture_loading_overlay(
        &self,
        painter: &egui::Painter,
        canvas_rect: Rect,
        loaded: usize,
        total: usize,
    ) {
        let progress = if total == 0 {
            1.0
        } else {
            loaded as f32 / total as f32
        };
        let panel_size = vec2(340.0, 92.0);
        let panel = Rect::from_center_size(canvas_rect.center(), panel_size);
        let bar_bg = Rect::from_min_size(
            panel.left_bottom() + vec2(24.0, -34.0),
            vec2(panel.width() - 48.0, 8.0),
        );
        let bar_fill = Rect::from_min_size(
            bar_bg.min,
            vec2(bar_bg.width() * progress.clamp(0.0, 1.0), bar_bg.height()),
        );

        painter.rect_filled(
            canvas_rect,
            0.0,
            Color32::from_rgba_unmultiplied(12, 14, 13, 132),
        );
        painter.rect_filled(panel, 6.0, Color32::from_rgba_unmultiplied(25, 27, 24, 238));
        painter.rect_stroke(
            panel,
            6.0,
            Stroke::new(1.0, THEME_BORDER),
            StrokeKind::Outside,
        );
        painter.text(
            panel.center_top() + vec2(0.0, 20.0),
            egui::Align2::CENTER_CENTER,
            "地图素材加载中",
            egui::TextStyle::Button.resolve(&egui::Style::default()),
            THEME_TEXT,
        );
        painter.text(
            panel.center_top() + vec2(0.0, 44.0),
            egui::Align2::CENTER_CENTER,
            format!("{loaded}/{total}"),
            egui::TextStyle::Small.resolve(&egui::Style::default()),
            THEME_MUTED_TEXT,
        );
        painter.rect_filled(
            bar_bg,
            3.0,
            Color32::from_rgba_unmultiplied(56, 58, 52, 255),
        );
        painter.rect_filled(bar_fill, 3.0, THEME_ACCENT);
    }

    pub(crate) fn handle_canvas_context_menu(
        &mut self,
        response: &egui::Response,
        canvas_rect: Rect,
        ctx: &EguiContext,
    ) {
        if response.secondary_clicked() {
            if let Some(pointer_pos) = ctx
                .input(|input| input.pointer.interact_pos())
                .or_else(|| response.hover_pos())
            {
                if let Some(selection) = self.hit_test_placed_item(canvas_rect, pointer_pos) {
                    if !self.current_selection_list().contains(&selection) {
                        self.set_single_selection(Some(selection));
                    }
                    if let Some(selection) = &self.selected_item {
                        self.active_layer = selection.layer;
                    }
                }
            }
        }

        response.context_menu(|ui| {
            let selections = self.current_selection_list();
            let Some(selection) = self.selected_item.clone() else {
                ui.label("No selection");
                return;
            };

            if selections.len() > 1 {
                ui.label(format!("已选 {} 个对象", selections.len()));
            } else {
                ui.label(selection.label());
                if ui.button("水平翻转").clicked() {
                    self.flip_selected_item();
                    ui.close();
                }
                if ui.button("右转 90").clicked() {
                    self.rotate_selected_item(90);
                    ui.close();
                }
                if ui.button("左转 90").clicked() {
                    self.rotate_selected_item(-90);
                    ui.close();
                }
                if ui.button("重置变换").clicked() {
                    self.reset_selected_transform();
                    ui.close();
                }
            }
            ui.separator();
            if ui.button("复制").clicked() {
                self.copy_selected_item();
                ui.close();
            }
            if ui.button("复制一份").clicked() {
                self.duplicate_selected_item();
                ui.close();
            }
            if ui.button("删除").clicked() {
                self.delete_current_selection();
                ui.close();
            }
            if selection.layer == LayerKind::Zones {
                ui.separator();
                if ui.button("删除附近顶点").clicked() {
                    if let Some(pointer_pos) = ctx
                        .input(|input| input.pointer.interact_pos())
                        .or_else(|| response.hover_pos())
                    {
                        self.delete_zone_vertex_near(canvas_rect, &selection.id, pointer_pos);
                    }
                    ui.close();
                }
            }
        });
    }

    pub(crate) fn apply_canvas_input(&mut self, response: &egui::Response, ctx: &EguiContext) {
        let space_down = ctx.input(|input| input.key_down(Key::Space));
        let middle_pan = ctx.input(|input| input.pointer.button_down(egui::PointerButton::Middle))
            && response.is_pointer_button_down_on();
        let panning = self.tool == ToolKind::Pan || space_down || middle_pan;

        if panning && (response.dragged() || middle_pan) {
            let pointer_delta = ctx.input(|input| input.pointer.delta());
            self.pan += pointer_delta;
        }

        if response.hovered() {
            let scroll = ctx.input(|input| input.smooth_scroll_delta.y);
            if scroll.abs() > f32::EPSILON {
                let factor = (1.0_f32 + scroll * 0.001).clamp(0.75, 1.25);
                self.zoom = (self.zoom * factor).clamp(0.25, 4.0);
            }
        }
    }

    pub(crate) fn handle_canvas_placement(
        &mut self,
        response: &egui::Response,
        canvas_rect: Rect,
        ctx: &EguiContext,
    ) {
        let space_down = ctx.input(|input| input.key_down(Key::Space));
        let middle_button_active = ctx.input(|input| {
            input.pointer.button_down(egui::PointerButton::Middle)
                || input.pointer.button_released(egui::PointerButton::Middle)
        });
        if matches!(self.tool, ToolKind::Select | ToolKind::Pan | ToolKind::Zoom)
            || space_down
            || middle_button_active
            || response.drag_started_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Middle)
            || response.clicked_by(egui::PointerButton::Middle)
        {
            return;
        }

        let primary_down = ctx.input(|input| input.pointer.primary_down());
        let continuous_paint = self.tool == ToolKind::Erase
            || self.tool == ToolKind::Collision
            || (self.tool == ToolKind::Brush
                && matches!(self.active_layer, LayerKind::Ground | LayerKind::Collision));
        let should_place = if self.tool == ToolKind::Rectangle {
            response.drag_started()
                || response.dragged()
                || response.drag_stopped()
                || response.clicked()
        } else if self.tool == ToolKind::Stamp {
            response.drag_started()
                || response.dragged()
                || response.drag_stopped()
                || response.clicked()
        } else if continuous_paint {
            primary_down && (response.hovered() || response.dragged())
        } else {
            response.clicked()
        };

        if !should_place {
            return;
        }

        let Some(mouse) = ctx
            .input(|input| input.pointer.interact_pos())
            .or_else(|| response.hover_pos())
        else {
            return;
        };
        let Some([tile_x, tile_y]) = self.screen_to_tile(canvas_rect, mouse) else {
            return;
        };
        let raw_map_pos = self
            .screen_to_map_position(canvas_rect, mouse)
            .unwrap_or([tile_x as f32, tile_y as f32]);

        if self.tool == ToolKind::Zone {
            self.handle_zone_tool(response, raw_map_pos, ctx.input(|input| input.modifiers));
            return;
        }

        if self.tool == ToolKind::Rectangle {
            self.handle_rectangle_tool(response, tile_x, tile_y);
            return;
        }

        if self.tool == ToolKind::Stamp {
            self.handle_stamp_tool(response, tile_x, tile_y);
            return;
        }

        if self.tool == ToolKind::Eyedropper {
            if let Some(selection) = self.hit_test_placed_item(canvas_rect, mouse) {
                if let Some(asset_id) = self.asset_for_selection(&selection) {
                    self.selected_asset = Some(asset_id.clone());
                    self.clear_selection();
                    self.active_layer = selection.layer;
                    self.tool = ToolKind::Brush;
                    if selection.layer == LayerKind::Ground {
                        if let Some([width, height]) =
                            self.ground_size_for_selection_id(&selection.id)
                        {
                            self.ground_footprint_w = width;
                            self.ground_footprint_h = height;
                        }
                    }
                    self.status = format!("吸取素材 {}", asset_id);
                }
            }
            return;
        }

        if self.active_layer_locked() {
            self.status = format!("{} 已锁定", self.active_layer.zh_label());
            return;
        }

        if self.tool == ToolKind::Erase {
            if self.active_layer == LayerKind::Collision {
                let modifiers = ctx.input(|input| input.modifiers);
                let [x, y] = self.snapped_collision_position(raw_map_pos, modifiers);
                let [width, height] = self.clamped_collision_brush_at(x, y);
                self.push_undo_snapshot();
                self.document.erase_collision_rect(x, y, width, height);
                self.mark_dirty();
                self.status = format!("Erased collision {:.2}, {:.2}", x, y);
                return;
            }

            let [width, height] = self.clamped_ground_footprint_at(tile_x, tile_y);
            self.push_undo_snapshot();
            self.erase_brush_at(tile_x, tile_y);
            if self.active_layer == LayerKind::Ground {
                self.autotile_ground_near_rect(tile_x, tile_y, width, height);
            }
            self.mark_dirty();
            self.status = format!("Erased {}, {}", tile_x, tile_y);
            return;
        }

        if self.tool == ToolKind::Collision || self.active_layer == LayerKind::Collision {
            let modifiers = ctx.input(|input| input.modifiers);
            let [x, y] = self.snapped_collision_position(raw_map_pos, modifiers);
            self.push_undo_snapshot();
            self.paint_collision_brush(x, y);
            self.mark_dirty();
            self.status = format!("Collision {:.2}, {:.2}", x, y);
            return;
        }

        if self.tool == ToolKind::Bucket {
            if self.active_layer != LayerKind::Ground {
                self.status = "油漆桶目前用于地表图层".to_owned();
                return;
            }
            let Some(asset_id) = self.selected_asset().map(|asset| asset.id.clone()) else {
                self.status = "油漆桶需要先选择地表素材".to_owned();
                return;
            };
            self.push_undo_snapshot();
            let filled = self.bucket_fill_ground(tile_x, tile_y, &asset_id);
            if filled > 0 {
                self.autotile_all_ground();
                self.mark_dirty();
            }
            self.status = format!("油漆桶填充 {} 格", filled);
            return;
        }

        let modifiers = ctx.input(|input| input.modifiers);
        let Some((asset_id, entity_type, place_pos)) = self.selected_asset().map(|asset| {
            (
                asset.id.clone(),
                asset.entity_type.clone(),
                self.snapped_map_position(raw_map_pos, Some(asset), modifiers),
            )
        }) else {
            self.status = "Select an asset first".to_owned();
            return;
        };
        self.clear_selection();
        self.push_undo_snapshot();

        let placed_ground_size = match self.active_layer {
            LayerKind::Ground => {
                let [width, height] = self.selected_ground_footprint_at(tile_x, tile_y);
                self.paint_ground_brush(tile_x, tile_y, &asset_id);
                self.autotile_ground_near_rect(tile_x, tile_y, width, height);
                Some([width, height])
            }
            LayerKind::Decals => {
                self.document
                    .place_decal(&asset_id, place_pos[0], place_pos[1]);
                None
            }
            LayerKind::Objects => {
                self.document
                    .place_object(&asset_id, place_pos[0], place_pos[1]);
                None
            }
            LayerKind::Entities => {
                let entity_type = entity_type.unwrap_or_else(|| "Decoration".to_owned());
                self.document
                    .place_entity(&asset_id, &entity_type, place_pos[0], place_pos[1]);
                None
            }
            LayerKind::Zones => {
                self.document
                    .place_decal(&asset_id, place_pos[0], place_pos[1]);
                None
            }
            LayerKind::Collision => unreachable!(),
        };

        self.mark_dirty();
        self.status = if let Some([width, height]) = placed_ground_size {
            format!(
                "Placed {} {}x{} at {}, {}",
                asset_id, width, height, tile_x, tile_y
            )
        } else {
            format!("Placed {} at {}, {}", asset_id, tile_x, tile_y)
        };
    }

    pub(crate) fn handle_canvas_selection(
        &mut self,
        response: &egui::Response,
        canvas_rect: Rect,
        ctx: &EguiContext,
    ) {
        let space_down = ctx.input(|input| input.key_down(Key::Space));
        let middle_button_active = ctx.input(|input| {
            input.pointer.button_down(egui::PointerButton::Middle)
                || input.pointer.button_released(egui::PointerButton::Middle)
        });
        if self.tool != ToolKind::Select
            || space_down
            || middle_button_active
            || response.drag_started_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Middle)
            || response.clicked_by(egui::PointerButton::Middle)
        {
            return;
        }

        let Some(pointer_pos) = ctx
            .input(|input| input.pointer.interact_pos())
            .or_else(|| response.hover_pos())
        else {
            return;
        };

        if response.drag_started() {
            let modifiers = ctx.input(|input| input.modifiers);
            let current = self.current_selection_list();
            self.resize_drag = None;
            self.zone_vertex_drag = None;
            self.selection_marquee = None;
            self.multi_move_drag = None;

            if current.len() == 1 {
                let selection = current[0].clone();
                if selection.layer == LayerKind::Zones {
                    if let Some(vertex_index) =
                        self.zone_vertex_hit(canvas_rect, &selection.id, pointer_pos)
                    {
                        self.zone_vertex_drag = Some(ZoneVertexDrag {
                            zone_id: selection.id.clone(),
                            vertex_index,
                        });
                        if !self.layer_state(LayerKind::Zones).locked {
                            self.push_undo_snapshot();
                        }
                        return;
                    }
                }

                if self.resize_handle_hit(canvas_rect, &selection, pointer_pos) {
                    self.resize_drag = Some(ResizeDrag {
                        selection: selection.clone(),
                    });
                    if !self.layer_state(selection.layer).locked {
                        self.push_undo_snapshot();
                    }
                    return;
                }
            }

            let hit = self.hit_test_placed_item(canvas_rect, pointer_pos);
            let additive = modifiers.shift || modifiers.command || modifiers.ctrl;
            if let Some(selection) = hit {
                self.active_layer = selection.layer;
                if additive {
                    self.toggle_selection(selection);
                    self.status = format!("已选中 {} 个对象", self.current_selection_list().len());
                    return;
                }

                if !current.contains(&selection) {
                    self.set_single_selection(Some(selection));
                }

                let selections = self.current_selection_list();
                let origins = self.move_origins_for_selection(&selections);
                if origins
                    .iter()
                    .any(|origin| !self.layer_state(origin.layer()).locked)
                {
                    if let Some(start) = self.screen_to_map_position(canvas_rect, pointer_pos) {
                        self.push_undo_snapshot();
                        self.multi_move_drag = Some(MultiMoveDrag { start, origins });
                    }
                }
                self.status = if selections.len() > 1 {
                    format!("已选中 {} 个对象", selections.len())
                } else {
                    format!("Selected {}", selections[0].label())
                };
            } else {
                if !additive {
                    self.clear_selection();
                }
                self.selection_marquee = Some(SelectionMarquee {
                    start: pointer_pos,
                    current: pointer_pos,
                    additive,
                });
                self.status = "框选中".to_owned();
            }
        }

        if response.dragged() && ctx.input(|input| input.pointer.primary_down()) {
            if let Some(marquee) = &mut self.selection_marquee {
                marquee.current = pointer_pos;
                return;
            }

            if let Some(drag) = self.zone_vertex_drag.clone() {
                if !self.layer_state(LayerKind::Zones).locked {
                    if let Some(raw) = self.screen_to_map_position(canvas_rect, pointer_pos) {
                        let point = self.snapped_map_position(
                            raw,
                            None,
                            ctx.input(|input| input.modifiers),
                        );
                        self.move_zone_vertex(&drag.zone_id, drag.vertex_index, point);
                        self.mark_dirty();
                        self.status = format!("移动区域顶点 #{}", drag.vertex_index);
                    }
                }
                return;
            }

            if let Some(drag) = self.multi_move_drag.clone() {
                if let Some(raw_pos) = self.screen_to_map_position(canvas_rect, pointer_pos) {
                    let modifiers = ctx.input(|input| input.modifiers);
                    let raw_delta = [raw_pos[0] - drag.start[0], raw_pos[1] - drag.start[1]];
                    let delta = snapped_delta(raw_delta, modifiers);
                    self.apply_multi_move(&drag.origins, delta);
                    self.mark_dirty();
                    self.status = format!("移动 {} 个对象", drag.origins.len());
                }
                return;
            }

            if let Some(resize) = self.resize_drag.clone() {
                self.resize_selected_item(canvas_rect, &resize.selection, pointer_pos, ctx);
                self.mark_dirty();
                self.status = format!("Resized {}", resize.selection.label());
                return;
            }
            let Some(selection) = self.selected_item.clone() else {
                return;
            };
            if self.layer_state(selection.layer).locked {
                self.status = format!("{} 已锁定", selection.layer.zh_label());
                return;
            }
            let raw_pos = self
                .screen_to_map_position(canvas_rect, pointer_pos)
                .unwrap_or([0.0, 0.0]);
            let modifiers = ctx.input(|input| input.modifiers);
            let snapped_pos = if selection.layer == LayerKind::Ground {
                self.screen_to_tile(canvas_rect, pointer_pos)
                    .map(|[x, y]| [x as f32, y as f32])
                    .unwrap_or(raw_pos)
            } else {
                let asset_id = self.asset_for_selection(&selection);
                let asset = asset_id.as_deref().and_then(|id| self.registry.get(id));
                self.snapped_map_position(raw_pos, asset, modifiers)
            };

            let moved_ground = self.move_selected_item(&selection, snapped_pos[0], snapped_pos[1]);
            if selection.layer == LayerKind::Ground {
                if let Some([new_x, new_y]) = moved_ground {
                    self.set_single_selection(Some(SelectedItem {
                        layer: LayerKind::Ground,
                        id: ground_selection_id(new_x, new_y),
                    }));
                    self.mark_dirty();
                    self.status = format!("Moved {} to {}, {}", selection.label(), new_x, new_y);
                    return;
                }
            }
            self.mark_dirty();
            self.status = format!(
                "Moved {} to {:.2}, {:.2}",
                selection.label(),
                snapped_pos[0],
                snapped_pos[1]
            );
            return;
        }

        if response.drag_stopped() {
            if let Some(marquee) = self.selection_marquee.take() {
                let rect = Rect::from_two_pos(marquee.start, marquee.current);
                if rect.width().abs() > 4.0 || rect.height().abs() > 4.0 {
                    let mut selections = if marquee.additive {
                        self.current_selection_list()
                    } else {
                        Vec::new()
                    };
                    selections.extend(self.selections_in_screen_rect(canvas_rect, rect));
                    self.set_selection(selections);
                    self.status = format!("框选 {} 个对象", self.current_selection_list().len());
                }
            }
            self.multi_move_drag = None;
            self.resize_drag = None;
            self.zone_vertex_drag = None;
        }

        if response.clicked() {
            let modifiers = ctx.input(|input| input.modifiers);
            if let Some(selection) = self.hit_test_placed_item(canvas_rect, pointer_pos) {
                if modifiers.shift || modifiers.command || modifiers.ctrl {
                    self.toggle_selection(selection);
                } else {
                    self.set_single_selection(Some(selection));
                }
                if let Some(selection) = self.selected_item.clone() {
                    self.active_layer = selection.layer;
                }
                let count = self.current_selection_list().len();
                self.status = if count > 1 {
                    format!("已选中 {count} 个对象")
                } else {
                    self.selected_item
                        .as_ref()
                        .map(|selection| format!("Selected {}", selection.label()))
                        .unwrap_or_else(|| "No object selected".to_owned())
                };
            } else {
                if !(modifiers.shift || modifiers.command || modifiers.ctrl) {
                    self.clear_selection();
                }
                self.status = "No object selected".to_owned();
            }
        }
    }

    pub(crate) fn handle_rectangle_tool(
        &mut self,
        response: &egui::Response,
        tile_x: i32,
        tile_y: i32,
    ) {
        if !matches!(self.active_layer, LayerKind::Ground | LayerKind::Collision) {
            self.status = "矩形工具目前用于地表和碰撞图层".to_owned();
            return;
        }
        if self.active_layer_locked() {
            self.status = format!("{} 已锁定", self.active_layer.zh_label());
            return;
        }

        if response.drag_started() || self.rectangle_drag_start.is_none() {
            self.rectangle_drag_start = Some([tile_x, tile_y]);
        }
        self.rectangle_drag_current = Some([tile_x, tile_y]);

        if response.drag_stopped() || response.clicked() {
            let start = self.rectangle_drag_start.unwrap_or([tile_x, tile_y]);
            let end = self.rectangle_drag_current.unwrap_or([tile_x, tile_y]);
            self.apply_rectangle_tool(start, end);
            self.rectangle_drag_start = None;
            self.rectangle_drag_current = None;
        }
    }

    pub(crate) fn handle_stamp_tool(
        &mut self,
        response: &egui::Response,
        tile_x: i32,
        tile_y: i32,
    ) {
        if response.drag_started() {
            self.stamp_capture_drag = Some(StampCaptureDrag {
                start: [tile_x, tile_y],
                current: [tile_x, tile_y],
            });
            self.status = "拖拽框选 Stamp 区域".to_owned();
            return;
        }

        if response.dragged() {
            if let Some(drag) = &mut self.stamp_capture_drag {
                drag.current = [tile_x, tile_y];
                self.status = "拖拽框选 Stamp 区域".to_owned();
            }
            return;
        }

        if response.drag_stopped() {
            if let Some(drag) = self.stamp_capture_drag.take() {
                self.capture_stamp_from_tile_rect(drag.start, drag.current);
            }
            return;
        }

        if !response.clicked() {
            return;
        }

        if self.stamp_pattern.is_none() {
            self.status = "先用盖章工具拖拽框选一片地图生成 Stamp".to_owned();
            return;
        }

        self.paste_stamp_at(tile_x, tile_y);
    }

    pub(crate) fn create_stamp_from_selection(&mut self) {
        let selections = self.current_selection_list();
        if selections.is_empty() {
            self.status = "请先选择要做成 Stamp 的对象".to_owned();
            return;
        }

        let items = selections
            .iter()
            .filter_map(|selection| self.stamp_item_for_selection(selection))
            .collect::<Vec<_>>();
        let Some(pattern) = self.normalized_stamp_pattern(items) else {
            self.status = "当前选择不能生成 Stamp".to_owned();
            return;
        };

        let count = pattern.item_count();
        self.stamp_pattern = Some(pattern);
        self.tool = ToolKind::Stamp;
        self.status = format!("已从选择生成 Stamp：{} 个对象", count);
    }

    pub(crate) fn clear_stamp_pattern(&mut self) {
        self.stamp_pattern = None;
        self.stamp_capture_drag = None;
        self.status = "已清空 Stamp".to_owned();
    }

    fn capture_stamp_from_tile_rect(&mut self, start: [i32; 2], end: [i32; 2]) {
        let [min_x, min_y, max_x, max_y] = normalized_tile_rect(start, end);
        let pattern = self.stamp_pattern_from_tile_rect(min_x, min_y, max_x, max_y);
        if pattern.items.is_empty() {
            self.status = "框选区域里没有可盖章对象".to_owned();
            return;
        }

        let count = pattern.item_count();
        let width = pattern.width;
        let height = pattern.height;
        self.stamp_pattern = Some(pattern);
        self.status = format!("已生成 Stamp：{}x{}，{} 个对象", width, height, count);
    }

    fn stamp_pattern_from_tile_rect(
        &self,
        min_x: i32,
        min_y: i32,
        max_x: i32,
        max_y: i32,
    ) -> StampPattern {
        let mut items = Vec::new();

        if self.layer_state(LayerKind::Ground).visible {
            for tile in &self.document.layers.ground {
                if tile.x >= min_x && tile.x <= max_x && tile.y >= min_y && tile.y <= max_y {
                    let mut tile = tile.clone();
                    tile.x -= min_x;
                    tile.y -= min_y;
                    items.push(StampItem::Ground(tile));
                }
            }
        }

        if self.layer_state(LayerKind::Decals).visible {
            for instance in &self.document.layers.decals {
                if instance_anchor_in_rect(instance.x, instance.y, min_x, min_y, max_x, max_y) {
                    let mut instance = instance.clone();
                    instance.x -= min_x as f32;
                    instance.y -= min_y as f32;
                    items.push(StampItem::Decal(instance));
                }
            }
        }

        if self.layer_state(LayerKind::Objects).visible {
            for instance in &self.document.layers.objects {
                if instance_anchor_in_rect(instance.x, instance.y, min_x, min_y, max_x, max_y) {
                    let mut instance = instance.clone();
                    instance.x -= min_x as f32;
                    instance.y -= min_y as f32;
                    items.push(StampItem::Object(instance));
                }
            }
        }

        if self.layer_state(LayerKind::Entities).visible {
            for instance in &self.document.layers.entities {
                if instance_anchor_in_rect(instance.x, instance.y, min_x, min_y, max_x, max_y) {
                    let mut instance = instance.clone();
                    instance.x -= min_x as f32;
                    instance.y -= min_y as f32;
                    items.push(StampItem::Entity(instance));
                }
            }
        }

        StampPattern {
            width: max_x - min_x + 1,
            height: max_y - min_y + 1,
            items,
        }
    }

    fn stamp_item_for_selection(&self, selection: &SelectedItem) -> Option<StampItem> {
        match self.clipboard_for_selection(selection)? {
            ClipboardItem::Ground(tile) => Some(StampItem::Ground(tile)),
            ClipboardItem::Decal(instance) => Some(StampItem::Decal(instance)),
            ClipboardItem::Object(instance) => Some(StampItem::Object(instance)),
            ClipboardItem::Entity(instance) => Some(StampItem::Entity(instance)),
            ClipboardItem::Zone(_) => None,
        }
    }

    fn normalized_stamp_pattern(&self, mut items: Vec<StampItem>) -> Option<StampPattern> {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for item in &items {
            let [item_min_x, item_min_y, item_max_x, item_max_y] = stamp_item_bounds(item)?;
            min_x = min_x.min(item_min_x);
            min_y = min_y.min(item_min_y);
            max_x = max_x.max(item_max_x);
            max_y = max_y.max(item_max_y);
        }

        if min_x == i32::MAX || min_y == i32::MAX {
            return None;
        }

        for item in &mut items {
            offset_stamp_item(item, -min_x, -min_y);
        }

        Some(StampPattern {
            width: (max_x - min_x).max(1),
            height: (max_y - min_y).max(1),
            items,
        })
    }

    fn paste_stamp_at(&mut self, x: i32, y: i32) {
        let Some(pattern) = self.stamp_pattern.clone() else {
            self.status = "Stamp 为空".to_owned();
            return;
        };
        if pattern
            .items
            .iter()
            .all(|item| self.layer_state(item.layer()).locked)
        {
            self.status = "Stamp 涉及的图层都已锁定".to_owned();
            return;
        }

        self.push_undo_snapshot();
        self.clear_selection();
        let mut next_selection = Vec::new();
        let mut placed = 0usize;
        let mut skipped = 0usize;

        for item in pattern.items {
            match item {
                StampItem::Ground(mut tile) => {
                    if self.layer_state(LayerKind::Ground).locked {
                        skipped += 1;
                        continue;
                    }
                    tile.x += x;
                    tile.y += y;
                    if tile.x < 0
                        || tile.y < 0
                        || tile.x >= self.document.width as i32
                        || tile.y >= self.document.height as i32
                    {
                        skipped += 1;
                        continue;
                    }
                    let width = tile.w.max(1).min(self.document.width as i32 - tile.x);
                    let height = tile.h.max(1).min(self.document.height as i32 - tile.y);
                    if width <= 0 || height <= 0 {
                        skipped += 1;
                        continue;
                    }
                    self.place_stamp_ground_tile(&tile, width, height);
                    next_selection.push(SelectedItem {
                        layer: LayerKind::Ground,
                        id: ground_selection_id(tile.x, tile.y),
                    });
                    placed += 1;
                }
                StampItem::Decal(mut instance) => {
                    if self.layer_state(LayerKind::Decals).locked {
                        skipped += 1;
                        continue;
                    }
                    instance.x += x as f32;
                    instance.y += y as f32;
                    if !self.map_position_in_bounds(instance.x, instance.y) {
                        skipped += 1;
                        continue;
                    }
                    instance.id = next_editor_object_id("decal", &self.document.layers.decals);
                    let id = instance.id.clone();
                    self.document.layers.decals.push(instance);
                    next_selection.push(SelectedItem {
                        layer: LayerKind::Decals,
                        id,
                    });
                    placed += 1;
                }
                StampItem::Object(mut instance) => {
                    if self.layer_state(LayerKind::Objects).locked {
                        skipped += 1;
                        continue;
                    }
                    instance.x += x as f32;
                    instance.y += y as f32;
                    if !self.map_position_in_bounds(instance.x, instance.y) {
                        skipped += 1;
                        continue;
                    }
                    instance.id = next_editor_object_id("obj", &self.document.layers.objects);
                    let id = instance.id.clone();
                    self.document.layers.objects.push(instance);
                    next_selection.push(SelectedItem {
                        layer: LayerKind::Objects,
                        id,
                    });
                    placed += 1;
                }
                StampItem::Entity(mut instance) => {
                    if self.layer_state(LayerKind::Entities).locked {
                        skipped += 1;
                        continue;
                    }
                    instance.x += x as f32;
                    instance.y += y as f32;
                    if !self.map_position_in_bounds(instance.x, instance.y) {
                        skipped += 1;
                        continue;
                    }
                    instance.id = next_editor_entity_id("ent", &self.document.layers.entities);
                    let id = instance.id.clone();
                    self.document.layers.entities.push(instance);
                    next_selection.push(SelectedItem {
                        layer: LayerKind::Entities,
                        id,
                    });
                    placed += 1;
                }
            }
        }

        if placed == 0 {
            self.status = "Stamp 没有可放置对象，可能越界或图层已锁定".to_owned();
            return;
        }

        self.set_selection(next_selection);
        self.mark_dirty();
        self.status = if skipped > 0 {
            format!("Stamp 已放置 {} 个对象，跳过 {}", placed, skipped)
        } else {
            format!("Stamp 已放置 {} 个对象", placed)
        };
    }

    fn place_stamp_ground_tile(&mut self, tile: &content::TileInstance, width: i32, height: i32) {
        for yy in tile.y..tile.y + height {
            for xx in tile.x..tile.x + width {
                self.document.erase_at(LayerKind::Ground, xx, yy);
            }
        }
        self.document
            .place_tile_sized(&tile.asset, tile.x, tile.y, width, height);
        if let Some(target) = self
            .document
            .layers
            .ground
            .iter_mut()
            .find(|target| target.x == tile.x && target.y == tile.y)
        {
            target.flip_x = tile.flip_x;
            target.rotation = tile.rotation;
        }
    }

    fn map_position_in_bounds(&self, x: f32, y: f32) -> bool {
        x >= 0.0 && y >= 0.0 && x < self.document.width as f32 && y < self.document.height as f32
    }

    pub(crate) fn handle_zone_tool(
        &mut self,
        response: &egui::Response,
        raw_pos: [f32; 2],
        modifiers: Modifiers,
    ) {
        if self.layer_state(LayerKind::Zones).locked {
            self.status = "区域图层已锁定".to_owned();
            return;
        }
        if !response.clicked() && !response.double_clicked() {
            return;
        }

        let point = self.snapped_zone_position(raw_pos, modifiers);
        if response.double_clicked() {
            if self.zone_draft_points.len() >= 2 {
                self.finish_zone_draft();
            }
            return;
        }

        if self.zone_draft_points.len() >= 3
            && distance_sq(point, self.zone_draft_points[0]) <= 0.20 * 0.20
        {
            self.finish_zone_draft();
            return;
        }

        self.zone_draft_points.push(point);
        self.active_layer = LayerKind::Zones;
        self.status = format!(
            "区域点 {}（2 点生成碰撞线，3 点以上生成区域；双击完成）",
            self.zone_draft_points.len()
        );
    }

    pub(crate) fn finish_zone_draft(&mut self) {
        if self.zone_draft_points.len() < 2 {
            self.status = "区域/线至少需要 2 个点".to_owned();
            return;
        }
        self.push_undo_snapshot();
        let id = next_editor_zone_id("zone", &self.document.layers.zones);
        let zone_type = if self.zone_draft_points.len() == 2 {
            "CollisionLine"
        } else {
            "Trigger"
        };
        self.document.layers.zones.push(content::ZoneInstance {
            id: id.clone(),
            zone_type: zone_type.to_owned(),
            points: self.zone_draft_points.clone(),
            hazard: None,
            prompt: None,
            objective: None,
            surface: None,
            unlock: None,
            transition: None,
        });
        self.zone_draft_points.clear();
        self.set_single_selection(Some(SelectedItem {
            layer: LayerKind::Zones,
            id,
        }));
        self.mark_dirty();
        self.status = if zone_type == "CollisionLine" {
            "碰撞线已创建".to_owned()
        } else {
            "区域已创建".to_owned()
        };
    }

    pub(crate) fn apply_rectangle_tool(&mut self, start: [i32; 2], end: [i32; 2]) {
        let min_x = start[0]
            .min(end[0])
            .clamp(0, self.document.width as i32 - 1);
        let max_x = start[0]
            .max(end[0])
            .clamp(0, self.document.width as i32 - 1);
        let min_y = start[1]
            .min(end[1])
            .clamp(0, self.document.height as i32 - 1);
        let max_y = start[1]
            .max(end[1])
            .clamp(0, self.document.height as i32 - 1);
        self.push_undo_snapshot();

        match self.active_layer {
            LayerKind::Ground => {
                let asset_id = self.selected_asset().map(|asset| asset.id.clone());
                let [step_w, step_h] = asset_id
                    .as_deref()
                    .and_then(|asset_id| self.registry.get(asset_id))
                    .map(|asset| self.asset_tile_footprint(asset))
                    .unwrap_or([1, 1]);
                for y in (min_y..=max_y).step_by(step_h.max(1) as usize) {
                    for x in (min_x..=max_x).step_by(step_w.max(1) as usize) {
                        if self.rectangle_erase_mode {
                            self.document.erase_at(LayerKind::Ground, x, y);
                        } else if let Some(asset_id) = &asset_id {
                            self.paint_ground_brush(x, y, asset_id);
                        } else {
                            self.status = "矩形填充需要先选择地表素材".to_owned();
                            return;
                        }
                    }
                }
                self.autotile_ground_near_rect(min_x, min_y, max_x - min_x + 1, max_y - min_y + 1);
                self.status = format!(
                    "矩形{}: {}x{}",
                    if self.rectangle_erase_mode {
                        "擦除"
                    } else {
                        "填充"
                    },
                    max_x - min_x + 1,
                    max_y - min_y + 1
                );
            }
            LayerKind::Collision => {
                let width = (max_x - min_x + 1) as f32;
                let height = (max_y - min_y + 1) as f32;
                if self.rectangle_erase_mode {
                    self.document
                        .erase_collision_rect(min_x as f32, min_y as f32, width, height);
                } else {
                    self.document
                        .place_collision_rect(min_x as f32, min_y as f32, width, height);
                }
                self.status = format!(
                    "矩形碰撞{}: {}x{}",
                    if self.rectangle_erase_mode {
                        "擦除"
                    } else {
                        "填充"
                    },
                    max_x - min_x + 1,
                    max_y - min_y + 1
                );
            }
            _ => {}
        }
        self.mark_dirty();
    }

    pub(crate) fn paint_ground_brush(&mut self, x: i32, y: i32, asset_id: &str) {
        let [width, height] = self
            .registry
            .get(asset_id)
            .map(|asset| self.clamped_tile_footprint_at(asset, x, y))
            .unwrap_or_else(|| self.clamped_ground_footprint_at(x, y));
        for yy in y..y + height {
            for xx in x..x + width {
                self.document.erase_at(LayerKind::Ground, xx, yy);
            }
        }
        self.document
            .place_tile_sized(asset_id, x, y, width, height);
    }

    pub(crate) fn paint_collision_brush(&mut self, x: f32, y: f32) {
        let [width, height] = self.clamped_collision_brush_at(x, y);
        self.document.place_collision_rect(x, y, width, height);
    }

    pub(crate) fn erase_brush_at(&mut self, x: i32, y: i32) {
        if self.active_layer == LayerKind::Collision {
            let [width, height] = self.clamped_collision_brush_at(x as f32, y as f32);
            self.document
                .erase_collision_rect(x as f32, y as f32, width, height);
            return;
        }

        let [width, height] = self.clamped_ground_footprint_at(x, y);
        for yy in y..y + height {
            for xx in x..x + width {
                self.document.erase_at(self.active_layer, xx, yy);
            }
        }
    }

    pub(crate) fn bucket_fill_ground(&mut self, x: i32, y: i32, asset_id: &str) -> usize {
        let target = self.ground_asset_at_cell(x, y);
        if target.as_deref() == Some(asset_id) {
            return 0;
        }

        let mut visited = vec![false; self.document.width as usize * self.document.height as usize];
        let mut queue = VecDeque::from([[x, y]]);
        let mut filled = 0;

        while let Some([cx, cy]) = queue.pop_front() {
            if cx < 0
                || cy < 0
                || cx >= self.document.width as i32
                || cy >= self.document.height as i32
            {
                continue;
            }
            let index = cy as usize * self.document.width as usize + cx as usize;
            if visited[index] {
                continue;
            }
            visited[index] = true;

            if self.ground_asset_at_cell(cx, cy) != target {
                continue;
            }

            self.document.erase_at(LayerKind::Ground, cx, cy);
            self.paint_ground_brush(cx, cy, asset_id);
            filled += 1;

            queue.extend([[cx + 1, cy], [cx - 1, cy], [cx, cy + 1], [cx, cy - 1]]);
        }

        filled
    }

    pub(crate) fn autotile_all_ground(&mut self) -> usize {
        if !self.terrain_autotile {
            return 0;
        }
        self.recalc_all_ground()
    }

    pub(crate) fn recalc_all_ground(&mut self) -> usize {
        let Some(rules) = self.terrain_rules_for_autotile() else {
            return 0;
        };
        let anchors = self
            .document
            .layers
            .ground
            .iter()
            .map(|tile| (tile.x, tile.y))
            .collect::<Vec<_>>();
        self.autotile_ground_anchors(&rules, anchors)
    }

    pub(crate) fn recalc_visible_ground(&mut self, canvas_rect: Rect) -> usize {
        let Some(rules) = self.terrain_rules_for_autotile() else {
            return 0;
        };
        let visible = self.visible_tile_bounds(canvas_rect);
        let anchors = self
            .document
            .layers
            .ground
            .iter()
            .filter(|tile| {
                tile_intersects_rect(
                    tile,
                    visible.min_x,
                    visible.min_y,
                    visible.max_x,
                    visible.max_y,
                )
            })
            .map(|tile| (tile.x, tile.y))
            .collect::<Vec<_>>();
        self.autotile_ground_anchors(&rules, anchors)
    }

    pub(crate) fn autotile_ground_near_rect(&mut self, x: i32, y: i32, w: i32, h: i32) -> usize {
        if !self.terrain_autotile {
            return 0;
        }
        let Some(rules) = self.terrain_rules_for_autotile() else {
            return 0;
        };
        let min_x = x.saturating_sub(1);
        let min_y = y.saturating_sub(1);
        let max_x = (x + w.max(1) + 1).min(self.document.width as i32);
        let max_y = (y + h.max(1) + 1).min(self.document.height as i32);
        let anchors = self
            .document
            .layers
            .ground
            .iter()
            .filter(|tile| tile_intersects_rect(tile, min_x, min_y, max_x, max_y))
            .map(|tile| (tile.x, tile.y))
            .collect::<Vec<_>>();

        self.autotile_ground_anchors(&rules, anchors)
    }

    fn terrain_rules_for_autotile(&self) -> Option<TerrainRules> {
        let rules = TerrainRules::from_assets(self.registry.assets());
        (!rules.is_empty()).then_some(rules)
    }

    fn autotile_ground_anchors(
        &mut self,
        rules: &TerrainRules,
        mut anchors: Vec<(i32, i32)>,
    ) -> usize {
        anchors.sort_unstable();
        anchors.dedup();

        let mut changed = 0;
        for (x, y) in anchors {
            changed += usize::from(self.autotile_ground_at_anchor(rules, x, y));
        }
        changed
    }

    fn autotile_ground_at_anchor(&mut self, rules: &TerrainRules, x: i32, y: i32) -> bool {
        let Some(tile) = self
            .document
            .layers
            .ground
            .iter()
            .find(|tile| tile.x == x && tile.y == y)
            .cloned()
        else {
            return false;
        };
        if rules.family_for_asset(&tile.asset).is_none() {
            return false;
        }
        let neighbors = TerrainNeighborFamilies {
            north: self.terrain_family_at_side(rules, &tile, DirectionSide::North),
            east: self.terrain_family_at_side(rules, &tile, DirectionSide::East),
            south: self.terrain_family_at_side(rules, &tile, DirectionSide::South),
            west: self.terrain_family_at_side(rules, &tile, DirectionSide::West),
        };
        let Some(choice) = rules.choice_for_neighbors(&tile.asset, &neighbors) else {
            return false;
        };
        if choice.asset_id == tile.asset && choice.rotation == tile.rotation {
            return false;
        }

        let Some(target) = self
            .document
            .layers
            .ground
            .iter_mut()
            .find(|target| target.x == x && target.y == y)
        else {
            return false;
        };
        target.asset = choice.asset_id;
        target.rotation = choice.rotation;
        true
    }

    fn terrain_family_at_side(
        &self,
        rules: &TerrainRules,
        tile: &content::TileInstance,
        side: DirectionSide,
    ) -> Option<String> {
        let width = tile.w.max(1);
        let height = tile.h.max(1);
        let [x, y] = match side {
            DirectionSide::North => [tile.x + width / 2, tile.y - 1],
            DirectionSide::East => [tile.x + width, tile.y + height / 2],
            DirectionSide::South => [tile.x + width / 2, tile.y + height],
            DirectionSide::West => [tile.x - 1, tile.y + height / 2],
        };
        if x < 0 || y < 0 || x >= self.document.width as i32 || y >= self.document.height as i32 {
            return None;
        }

        self.document
            .layers
            .ground
            .iter()
            .rev()
            .find(|candidate| tile_contains_cell(candidate, x, y))
            .and_then(|candidate| rules.family_for_asset(&candidate.asset))
            .map(str::to_owned)
    }

    pub(crate) fn ground_asset_at_cell(&self, x: i32, y: i32) -> Option<String> {
        self.document
            .layers
            .ground
            .iter()
            .rev()
            .find(|tile| {
                let width = tile.w.max(1);
                let height = tile.h.max(1);
                x >= tile.x && x < tile.x + width && y >= tile.y && y < tile.y + height
            })
            .map(|tile| tile.asset.clone())
    }

    pub(crate) fn resize_handle_hit(
        &self,
        canvas_rect: Rect,
        selection: &SelectedItem,
        pointer_pos: Pos2,
    ) -> bool {
        self.selection_screen_rect(canvas_rect, selection)
            .is_some_and(|rect| {
                resize_handle_rects(rect)
                    .iter()
                    .any(|handle| handle.contains(pointer_pos))
            })
    }

    pub(crate) fn resize_selected_item(
        &mut self,
        canvas_rect: Rect,
        selection: &SelectedItem,
        pointer_pos: Pos2,
        ctx: &EguiContext,
    ) {
        if self.layer_state(selection.layer).locked {
            return;
        }
        let Some((anchor, asset_id)) = self.selection_anchor_and_asset(canvas_rect, selection)
        else {
            return;
        };
        let Some(asset) = self.registry.get(&asset_id) else {
            return;
        };

        let delta_x = (pointer_pos.x - anchor.x).abs();
        let delta_y = (pointer_pos.y - anchor.y).abs();
        let screen_size = match asset.anchor {
            AnchorKind::TopLeft => vec2(delta_x.max(1.0), delta_y.max(1.0)),
            AnchorKind::Center => vec2((delta_x * 2.0).max(1.0), (delta_y * 2.0).max(1.0)),
            AnchorKind::BottomCenter => vec2((delta_x * 2.0).max(1.0), delta_y.max(1.0)),
        };
        let world_size = screen_size / self.zoom.max(0.01);
        let mut scale_x = (world_size.x / asset.default_size[0].max(1.0)).max(0.05);
        let mut scale_y = (world_size.y / asset.default_size[1].max(1.0)).max(0.05);
        let keep_aspect = self.lock_aspect_ratio || ctx.input(|input| input.modifiers.shift);
        if keep_aspect {
            let uniform = scale_x.max(scale_y);
            scale_x = uniform;
            scale_y = uniform;
        }

        self.set_scale_for_selection(selection, scale_x, scale_y);
    }

    pub(crate) fn selection_anchor_and_asset(
        &self,
        canvas_rect: Rect,
        selection: &SelectedItem,
    ) -> Option<(Pos2, String)> {
        let tile_size = self.document.tile_size as f32;
        match selection.layer {
            LayerKind::Decals => self
                .document
                .layers
                .decals
                .iter()
                .find(|instance| instance.id == selection.id)
                .and_then(|instance| {
                    let asset = self.registry.get(&instance.asset)?;
                    Some((
                        self.world_to_screen(
                            canvas_rect,
                            anchor_grid_to_world(tile_size, instance.x, instance.y, asset.anchor),
                        ),
                        instance.asset.clone(),
                    ))
                }),
            LayerKind::Objects => self
                .document
                .layers
                .objects
                .iter()
                .find(|instance| instance.id == selection.id)
                .and_then(|instance| {
                    let asset = self.registry.get(&instance.asset)?;
                    Some((
                        self.world_to_screen(
                            canvas_rect,
                            anchor_grid_to_world(tile_size, instance.x, instance.y, asset.anchor),
                        ),
                        instance.asset.clone(),
                    ))
                }),
            LayerKind::Entities => self
                .document
                .layers
                .entities
                .iter()
                .find(|instance| instance.id == selection.id)
                .and_then(|instance| {
                    let asset = self.registry.get(&instance.asset)?;
                    Some((
                        self.world_to_screen(
                            canvas_rect,
                            anchor_grid_to_world(tile_size, instance.x, instance.y, asset.anchor),
                        ),
                        instance.asset.clone(),
                    ))
                }),
            _ => None,
        }
    }

    pub(crate) fn set_scale_for_selection(
        &mut self,
        selection: &SelectedItem,
        scale_x: f32,
        scale_y: f32,
    ) {
        match selection.layer {
            LayerKind::Decals => {
                if let Some(instance) = self
                    .document
                    .layers
                    .decals
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.scale_x = scale_x;
                    instance.scale_y = scale_y;
                }
            }
            LayerKind::Objects => {
                if let Some(instance) = self
                    .document
                    .layers
                    .objects
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.scale_x = scale_x;
                    instance.scale_y = scale_y;
                }
            }
            LayerKind::Entities => {
                if let Some(instance) = self
                    .document
                    .layers
                    .entities
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.scale_x = scale_x;
                    instance.scale_y = scale_y;
                }
            }
            _ => {}
        }
    }

    pub(crate) fn zone_vertex_hit(
        &self,
        canvas_rect: Rect,
        zone_id: &str,
        pointer_pos: Pos2,
    ) -> Option<usize> {
        let zone = self
            .document
            .layers
            .zones
            .iter()
            .find(|zone| zone.id == zone_id)?;
        let tile_size = self.document.tile_size as f32;
        zone.points.iter().position(|point| {
            let screen = self.world_to_screen(
                canvas_rect,
                vec2(point[0] * tile_size, point[1] * tile_size),
            );
            Rect::from_center_size(screen, vec2(12.0, 12.0)).contains(pointer_pos)
        })
    }

    pub(crate) fn move_zone_vertex(&mut self, zone_id: &str, vertex_index: usize, point: [f32; 2]) {
        if let Some(zone) = self
            .document
            .layers
            .zones
            .iter_mut()
            .find(|zone| zone.id == zone_id)
        {
            if let Some(vertex) = zone.points.get_mut(vertex_index) {
                *vertex = point;
            }
        }
    }

    pub(crate) fn delete_zone_vertex_near(
        &mut self,
        canvas_rect: Rect,
        zone_id: &str,
        pointer_pos: Pos2,
    ) {
        if self.layer_state(LayerKind::Zones).locked {
            self.status = "区域图层已锁定".to_owned();
            return;
        }
        let Some(index) = self.zone_vertex_hit(canvas_rect, zone_id, pointer_pos) else {
            self.status = "附近没有区域顶点".to_owned();
            return;
        };
        let Some(zone) = self
            .document
            .layers
            .zones
            .iter()
            .find(|zone| zone.id == zone_id)
        else {
            return;
        };
        if zone.points.len() <= 3 {
            self.status = "区域至少需要保留 3 个点".to_owned();
            return;
        }
        self.push_undo_snapshot();
        if let Some(zone) = self
            .document
            .layers
            .zones
            .iter_mut()
            .find(|zone| zone.id == zone_id)
        {
            zone.points.remove(index);
            self.mark_dirty();
            self.status = format!("已删除区域顶点 #{index}");
        }
    }

    pub(crate) fn move_origins_for_selection(
        &self,
        selections: &[SelectedItem],
    ) -> Vec<MoveOrigin> {
        selections
            .iter()
            .filter_map(|selection| match selection.layer {
                LayerKind::Ground => {
                    let [x, y] = parse_ground_selection_id(&selection.id)?;
                    self.document
                        .layers
                        .ground
                        .iter()
                        .find(|tile| tile.x == x && tile.y == y)
                        .map(|tile| MoveOrigin::Ground {
                            selection: selection.clone(),
                            x: tile.x,
                            y: tile.y,
                        })
                }
                LayerKind::Decals => self
                    .document
                    .layers
                    .decals
                    .iter()
                    .find(|instance| instance.id == selection.id)
                    .map(|instance| MoveOrigin::ObjectLike {
                        selection: selection.clone(),
                        x: instance.x,
                        y: instance.y,
                    }),
                LayerKind::Objects => self
                    .document
                    .layers
                    .objects
                    .iter()
                    .find(|instance| instance.id == selection.id)
                    .map(|instance| MoveOrigin::ObjectLike {
                        selection: selection.clone(),
                        x: instance.x,
                        y: instance.y,
                    }),
                LayerKind::Entities => self
                    .document
                    .layers
                    .entities
                    .iter()
                    .find(|instance| instance.id == selection.id)
                    .map(|instance| MoveOrigin::ObjectLike {
                        selection: selection.clone(),
                        x: instance.x,
                        y: instance.y,
                    }),
                LayerKind::Zones => self
                    .document
                    .layers
                    .zones
                    .iter()
                    .find(|zone| zone.id == selection.id)
                    .map(|zone| MoveOrigin::Zone {
                        selection: selection.clone(),
                        points: zone.points.clone(),
                    }),
                LayerKind::Collision => None,
            })
            .collect()
    }

    pub(crate) fn apply_multi_move(&mut self, origins: &[MoveOrigin], delta: [f32; 2]) {
        let max_x = self.document.width as f32;
        let max_y = self.document.height as f32;
        let mut next_selection = Vec::with_capacity(origins.len());

        for origin in origins {
            if self.layer_state(origin.layer()).locked {
                continue;
            }

            match origin {
                MoveOrigin::Ground { selection, x, y } => {
                    let new_x = (*x as f32 + delta[0])
                        .round()
                        .clamp(0.0, self.document.width.saturating_sub(1) as f32)
                        as i32;
                    let new_y = (*y as f32 + delta[1])
                        .round()
                        .clamp(0.0, self.document.height.saturating_sub(1) as f32)
                        as i32;
                    let updated = self
                        .move_selected_item(selection, new_x as f32, new_y as f32)
                        .unwrap_or([new_x, new_y]);
                    next_selection.push(SelectedItem {
                        layer: LayerKind::Ground,
                        id: ground_selection_id(updated[0], updated[1]),
                    });
                }
                MoveOrigin::ObjectLike { selection, x, y } => {
                    let new_x = (x + delta[0]).clamp(0.0, max_x);
                    let new_y = (y + delta[1]).clamp(0.0, max_y);
                    self.move_selected_item(selection, new_x, new_y);
                    next_selection.push(selection.clone());
                }
                MoveOrigin::Zone { selection, points } => {
                    if let Some(zone) = self
                        .document
                        .layers
                        .zones
                        .iter_mut()
                        .find(|zone| zone.id == selection.id)
                    {
                        zone.points = points
                            .iter()
                            .map(|point| {
                                [
                                    (point[0] + delta[0]).clamp(0.0, max_x),
                                    (point[1] + delta[1]).clamp(0.0, max_y),
                                ]
                            })
                            .collect();
                    }
                    next_selection.push(selection.clone());
                }
            }
        }

        self.set_selection(next_selection);
    }

    pub(crate) fn alignable_selection_count(&self, selections: &[SelectedItem]) -> usize {
        selections
            .iter()
            .filter(|selection| {
                !self.layer_state(selection.layer).locked
                    && self.selection_map_bounds(selection).is_some()
            })
            .count()
    }

    pub(crate) fn distributable_selection_count(&self, selections: &[SelectedItem]) -> usize {
        self.alignable_selection_count(selections)
    }

    pub(crate) fn align_selected_items(&mut self, mode: BatchAlignMode) {
        let selections = self.current_selection_list();
        if selections.is_empty() {
            self.status = "请先选择对象".to_owned();
            return;
        }

        let candidates = selections
            .iter()
            .filter(|selection| !self.layer_state(selection.layer).locked)
            .filter_map(|selection| {
                self.selection_map_bounds(selection)
                    .map(|bounds| (selection.clone(), bounds))
            })
            .collect::<Vec<_>>();

        if candidates.len() < 2 {
            self.status = "至少选择两个可对齐对象".to_owned();
            return;
        }

        let mut group_bounds = candidates[0].1;
        for (_, bounds) in candidates.iter().skip(1) {
            group_bounds.include(*bounds);
        }
        let target = align_target(mode, group_bounds);
        let moves = candidates
            .into_iter()
            .filter_map(|(selection, bounds)| {
                let delta = align_delta(mode, bounds, target);
                is_meaningful_delta(delta).then_some((selection, delta))
            })
            .collect::<Vec<_>>();
        if moves.is_empty() {
            self.status = format!("所选对象已经{}", mode.label());
            return;
        }

        let mut next_selection = selections;
        let mut changed = 0;

        self.push_undo_snapshot();
        for (selection, delta) in moves {
            if let Some(updated) = self.offset_selection_by_delta(&selection, delta) {
                if let Some(index) = next_selection.iter().position(|item| item == &selection) {
                    next_selection[index] = updated;
                }
                changed += 1;
            }
        }

        if changed == 0 {
            self.status = format!("所选对象已经{}", mode.label());
            return;
        }

        self.set_selection(next_selection);
        self.mark_dirty();
        self.status = format!("已{} {} 个对象", mode.label(), changed);
    }

    pub(crate) fn distribute_selected_items(&mut self, mode: BatchDistributeMode) {
        let selections = self.current_selection_list();
        if selections.is_empty() {
            self.status = "请先选择对象".to_owned();
            return;
        }

        let candidates = selections
            .iter()
            .enumerate()
            .filter(|(_, selection)| !self.layer_state(selection.layer).locked)
            .filter_map(|(index, selection)| {
                self.selection_map_bounds(selection)
                    .map(|bounds| (index, bounds))
            })
            .collect::<Vec<_>>();

        if candidates.len() < 3 {
            self.status = "至少选择三个可分布对象".to_owned();
            return;
        }

        let moves = distribute_deltas(mode, candidates);
        if moves.is_empty() {
            self.status = format!("所选对象已经{}", mode.label());
            return;
        }

        let mut next_selection = selections;
        let mut changed = 0;

        self.push_undo_snapshot();
        for (index, delta) in moves {
            let selection = next_selection[index].clone();
            if let Some(updated) = self.offset_selection_by_delta(&selection, delta) {
                next_selection[index] = updated;
                changed += 1;
            }
        }

        if changed == 0 {
            self.status = format!("所选对象已经{}", mode.label());
            return;
        }

        self.set_selection(next_selection);
        self.mark_dirty();
        self.status = format!("已{} {} 个对象", mode.label(), changed);
    }

    pub(crate) fn nudge_current_selection(&mut self, delta: [f32; 2]) {
        let selections = self.current_selection_list();
        if selections.is_empty() {
            return;
        }

        let editable = selections
            .iter()
            .filter(|selection| {
                !self.layer_state(selection.layer).locked
                    && self.selection_map_bounds(selection).is_some()
            })
            .cloned()
            .collect::<Vec<_>>();

        if editable.is_empty() {
            self.status = "所选图层已锁定或不能微调".to_owned();
            return;
        }

        let mut next_selection = selections;
        let mut changed = 0;

        self.push_undo_snapshot();
        for selection in editable {
            if let Some(updated) = self.offset_selection_by_delta(&selection, delta) {
                if let Some(index) = next_selection.iter().position(|item| item == &selection) {
                    next_selection[index] = updated;
                }
                changed += 1;
            }
        }

        if changed == 0 {
            return;
        }

        self.set_selection(next_selection);
        self.mark_dirty();
        self.status = format!(
            "微调 {} 个对象：{:+.1}, {:+.1}",
            changed, delta[0], delta[1]
        );
    }

    pub(crate) fn replaceable_selection_count(
        &self,
        selections: &[SelectedItem],
        asset: &AssetEntry,
    ) -> usize {
        selections
            .iter()
            .filter(|selection| {
                !self.layer_state(selection.layer).locked
                    && self.selection_accepts_asset(selection, asset)
                    && self.asset_for_selection(selection).as_deref() != Some(asset.id.as_str())
            })
            .count()
    }

    pub(crate) fn replace_selected_assets_with_current(&mut self) {
        let selections = self.current_selection_list();
        if selections.is_empty() {
            self.status = "请先选择对象".to_owned();
            return;
        }

        let Some(asset) = self.selected_asset().cloned() else {
            self.status = "请先在素材库选择一个素材".to_owned();
            return;
        };

        let replaceable = selections
            .iter()
            .filter(|selection| {
                !self.layer_state(selection.layer).locked
                    && self.selection_accepts_asset(selection, &asset)
                    && self.asset_for_selection(selection).as_deref() != Some(asset.id.as_str())
            })
            .cloned()
            .collect::<Vec<_>>();

        if replaceable.is_empty() {
            self.status = format!("没有可替换为 {} 的选中对象", asset.id);
            return;
        }

        self.push_undo_snapshot();
        let mut changed = 0;
        for selection in &replaceable {
            if self.replace_asset_for_selection(selection, &asset) {
                changed += 1;
            }
        }

        if changed == 0 {
            self.status = format!("没有可替换为 {} 的选中对象", asset.id);
            return;
        }

        self.mark_dirty();
        self.status = format!("已用 {} 替换 {} 个对象", asset.id, changed);
    }

    pub(crate) fn editable_entity_selection_count(&self, selections: &[SelectedItem]) -> usize {
        selections
            .iter()
            .filter(|selection| {
                selection.layer == LayerKind::Entities
                    && !self.layer_state(selection.layer).locked
                    && self.entity_type_for_selection(selection).is_some()
            })
            .count()
    }

    pub(crate) fn common_entity_type_for_selection(
        &self,
        selections: &[SelectedItem],
    ) -> (String, bool) {
        common_text_value(selections.iter().filter_map(|selection| {
            if selection.layer != LayerKind::Entities || self.layer_state(selection.layer).locked {
                return None;
            }
            self.entity_type_for_selection(selection)
        }))
    }

    pub(crate) fn set_entity_type_for_selection(
        &mut self,
        selections: &[SelectedItem],
        entity_type: String,
    ) {
        let entity_type = entity_type.trim().to_owned();
        let targets = selections
            .iter()
            .filter(|selection| {
                selection.layer == LayerKind::Entities
                    && !self.layer_state(selection.layer).locked
                    && self.entity_type_for_selection(selection).as_deref() != Some(&entity_type)
            })
            .cloned()
            .collect::<Vec<_>>();

        if targets.is_empty() {
            self.status = "没有需要修改实体类型的对象".to_owned();
            return;
        }

        self.push_undo_snapshot();
        let mut changed = 0;
        for selection in &targets {
            if let Some(instance) = self
                .document
                .layers
                .entities
                .iter_mut()
                .find(|instance| instance.id == selection.id)
            {
                instance.entity_type = entity_type.clone();
                changed += 1;
            }
        }

        self.mark_dirty();
        self.status = format!("已批量设置 {} 个实体类型", changed);
    }

    pub(crate) fn unlockable_selection_count(&self, selections: &[SelectedItem]) -> usize {
        selections
            .iter()
            .filter(|selection| {
                matches!(selection.layer, LayerKind::Entities | LayerKind::Zones)
                    && !self.layer_state(selection.layer).locked
                    && self.unlock_for_selection(selection).is_some()
            })
            .count()
    }

    pub(crate) fn common_unlock_codex_for_selection(
        &self,
        selections: &[SelectedItem],
    ) -> (String, bool) {
        common_optional_text_value(self.unlockable_selections(selections).map(|selection| {
            self.unlock_for_selection(selection)
                .and_then(|unlock| normalized_optional(unlock.requires_codex_id.as_ref()))
        }))
    }

    pub(crate) fn common_unlock_item_for_selection(
        &self,
        selections: &[SelectedItem],
    ) -> (String, bool) {
        common_optional_text_value(self.unlockable_selections(selections).map(|selection| {
            self.unlock_for_selection(selection)
                .and_then(|unlock| normalized_optional(unlock.requires_item_id.as_ref()))
        }))
    }

    pub(crate) fn common_unlock_message_for_selection(
        &self,
        selections: &[SelectedItem],
    ) -> (String, bool) {
        common_optional_text_value(self.unlockable_selections(selections).map(|selection| {
            self.unlock_for_selection(selection)
                .and_then(|unlock| normalized_optional(unlock.locked_message.as_ref()))
        }))
    }

    pub(crate) fn set_unlock_codex_for_selection(
        &mut self,
        selections: &[SelectedItem],
        value: String,
    ) {
        self.set_unlock_field_for_selection(selections, BatchUnlockField::Codex, value);
    }

    pub(crate) fn set_unlock_item_for_selection(
        &mut self,
        selections: &[SelectedItem],
        value: String,
    ) {
        self.set_unlock_field_for_selection(selections, BatchUnlockField::Item, value);
    }

    pub(crate) fn set_unlock_message_for_selection(
        &mut self,
        selections: &[SelectedItem],
        value: String,
    ) {
        self.set_unlock_field_for_selection(selections, BatchUnlockField::Message, value);
    }

    pub(crate) fn clear_unlock_for_selection(&mut self, selections: &[SelectedItem]) {
        let targets = self
            .unlockable_selections(selections)
            .filter(|selection| {
                self.unlock_for_selection(selection)
                    .is_some_and(|unlock| !unlock.is_empty())
            })
            .cloned()
            .collect::<Vec<_>>();

        if targets.is_empty() {
            self.status = "所选对象没有解锁条件".to_owned();
            return;
        }

        self.push_undo_snapshot();
        let mut changed = 0;
        for selection in &targets {
            changed += usize::from(self.set_unlock_for_selection(selection, None));
        }
        self.mark_dirty();
        self.status = format!("已清除 {} 个解锁条件", changed);
    }

    fn selection_accepts_asset(&self, selection: &SelectedItem, asset: &AssetEntry) -> bool {
        expected_asset_kind_for_layer(selection.layer).is_some_and(|kind| kind == asset.kind)
    }

    fn replace_asset_for_selection(
        &mut self,
        selection: &SelectedItem,
        asset: &AssetEntry,
    ) -> bool {
        match selection.layer {
            LayerKind::Ground => {
                let Some([x, y]) = parse_ground_selection_id(&selection.id) else {
                    return false;
                };
                let Some(tile) = self
                    .document
                    .layers
                    .ground
                    .iter_mut()
                    .find(|tile| tile.x == x && tile.y == y)
                else {
                    return false;
                };
                if tile.asset == asset.id {
                    return false;
                }
                tile.asset = asset.id.clone();
                true
            }
            LayerKind::Decals => {
                let Some(instance) = self
                    .document
                    .layers
                    .decals
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                else {
                    return false;
                };
                if instance.asset == asset.id {
                    return false;
                }
                instance.asset = asset.id.clone();
                true
            }
            LayerKind::Objects => {
                let Some(instance) = self
                    .document
                    .layers
                    .objects
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                else {
                    return false;
                };
                if instance.asset == asset.id {
                    return false;
                }
                instance.asset = asset.id.clone();
                true
            }
            LayerKind::Entities => {
                let Some(instance) = self
                    .document
                    .layers
                    .entities
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                else {
                    return false;
                };
                if instance.asset == asset.id {
                    return false;
                }
                instance.asset = asset.id.clone();
                if let Some(entity_type) = &asset.entity_type {
                    instance.entity_type = entity_type.clone();
                }
                true
            }
            LayerKind::Zones | LayerKind::Collision => false,
        }
    }

    fn entity_type_for_selection(&self, selection: &SelectedItem) -> Option<String> {
        if selection.layer != LayerKind::Entities {
            return None;
        }
        self.document
            .layers
            .entities
            .iter()
            .find(|instance| instance.id == selection.id)
            .map(|instance| instance.entity_type.clone())
    }

    fn unlockable_selections<'a>(
        &'a self,
        selections: &'a [SelectedItem],
    ) -> impl Iterator<Item = &'a SelectedItem> + 'a {
        selections.iter().filter(|selection| {
            matches!(selection.layer, LayerKind::Entities | LayerKind::Zones)
                && !self.layer_state(selection.layer).locked
                && self.unlock_for_selection(selection).is_some()
        })
    }

    fn unlock_for_selection(&self, selection: &SelectedItem) -> Option<&UnlockRule> {
        match selection.layer {
            LayerKind::Entities => self
                .document
                .layers
                .entities
                .iter()
                .find(|instance| instance.id == selection.id)
                .map(|instance| instance.unlock.as_ref().unwrap_or(&EMPTY_UNLOCK_RULE)),
            LayerKind::Zones => self
                .document
                .layers
                .zones
                .iter()
                .find(|zone| zone.id == selection.id)
                .map(|zone| zone.unlock.as_ref().unwrap_or(&EMPTY_UNLOCK_RULE)),
            LayerKind::Ground | LayerKind::Decals | LayerKind::Objects | LayerKind::Collision => {
                None
            }
        }
    }

    fn set_unlock_field_for_selection(
        &mut self,
        selections: &[SelectedItem],
        field: BatchUnlockField,
        value: String,
    ) {
        let value = trimmed_optional(&value);
        let targets = self
            .unlockable_selections(selections)
            .filter(|selection| self.unlock_field_for_selection(selection, field) != value)
            .cloned()
            .collect::<Vec<_>>();

        if targets.is_empty() {
            self.status = "没有需要修改的解锁字段".to_owned();
            return;
        }

        self.push_undo_snapshot();
        let mut changed = 0;
        for selection in &targets {
            let mut unlock = self
                .unlock_for_selection(selection)
                .cloned()
                .unwrap_or_default();
            match field {
                BatchUnlockField::Codex => unlock.requires_codex_id = value.clone(),
                BatchUnlockField::Item => unlock.requires_item_id = value.clone(),
                BatchUnlockField::Message => unlock.locked_message = value.clone(),
            }
            let next = (!unlock.is_empty()).then_some(unlock);
            changed += usize::from(self.set_unlock_for_selection(selection, next));
        }

        self.mark_dirty();
        self.status = format!("已批量设置 {} 个解锁字段", changed);
    }

    fn unlock_field_for_selection(
        &self,
        selection: &SelectedItem,
        field: BatchUnlockField,
    ) -> Option<String> {
        let unlock = self.unlock_for_selection(selection)?;
        match field {
            BatchUnlockField::Codex => normalized_optional(unlock.requires_codex_id.as_ref()),
            BatchUnlockField::Item => normalized_optional(unlock.requires_item_id.as_ref()),
            BatchUnlockField::Message => normalized_optional(unlock.locked_message.as_ref()),
        }
    }

    fn set_unlock_for_selection(
        &mut self,
        selection: &SelectedItem,
        unlock: Option<UnlockRule>,
    ) -> bool {
        match selection.layer {
            LayerKind::Entities => {
                let Some(instance) = self
                    .document
                    .layers
                    .entities
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                else {
                    return false;
                };
                if instance.unlock == unlock {
                    return false;
                }
                instance.unlock = unlock;
                true
            }
            LayerKind::Zones => {
                let Some(zone) = self
                    .document
                    .layers
                    .zones
                    .iter_mut()
                    .find(|zone| zone.id == selection.id)
                else {
                    return false;
                };
                if zone.unlock == unlock {
                    return false;
                }
                zone.unlock = unlock;
                true
            }
            LayerKind::Ground | LayerKind::Decals | LayerKind::Objects | LayerKind::Collision => {
                false
            }
        }
    }

    fn selection_map_bounds(&self, selection: &SelectedItem) -> Option<SelectionMapBounds> {
        match selection.layer {
            LayerKind::Ground => {
                let [x, y] = parse_ground_selection_id(&selection.id)?;
                self.document
                    .layers
                    .ground
                    .iter()
                    .find(|tile| tile.x == x && tile.y == y)
                    .map(|tile| {
                        let width = tile.w.max(1) as f32;
                        let height = tile.h.max(1) as f32;
                        SelectionMapBounds::from_min_max(
                            tile.x as f32,
                            tile.y as f32,
                            tile.x as f32 + width,
                            tile.y as f32 + height,
                        )
                    })
            }
            LayerKind::Decals => self
                .document
                .layers
                .decals
                .iter()
                .find(|instance| instance.id == selection.id)
                .map(|instance| self.object_like_map_bounds(&instance.asset, instance)),
            LayerKind::Objects => self
                .document
                .layers
                .objects
                .iter()
                .find(|instance| instance.id == selection.id)
                .map(|instance| self.object_like_map_bounds(&instance.asset, instance)),
            LayerKind::Entities => self
                .document
                .layers
                .entities
                .iter()
                .find(|instance| instance.id == selection.id)
                .map(|instance| {
                    self.object_like_map_bounds_from_parts(
                        &instance.asset,
                        instance.x,
                        instance.y,
                        instance.scale_x,
                        instance.scale_y,
                    )
                }),
            LayerKind::Zones => self
                .document
                .layers
                .zones
                .iter()
                .find(|zone| zone.id == selection.id)
                .and_then(|zone| zone_map_bounds(&zone.points)),
            LayerKind::Collision => None,
        }
    }

    fn object_like_map_bounds(
        &self,
        asset_id: &str,
        instance: &content::ObjectInstance,
    ) -> SelectionMapBounds {
        self.object_like_map_bounds_from_parts(
            asset_id,
            instance.x,
            instance.y,
            instance.scale_x,
            instance.scale_y,
        )
    }

    fn object_like_map_bounds_from_parts(
        &self,
        asset_id: &str,
        x: f32,
        y: f32,
        scale_x: f32,
        scale_y: f32,
    ) -> SelectionMapBounds {
        let Some(asset) = self.registry.get(asset_id) else {
            return SelectionMapBounds::from_min_max(x, y, x + 1.0, y + 1.0);
        };
        let tile_size = self.document.tile_size.max(1) as f32;
        let width = asset.default_size[0] * scale_x.max(0.05) / tile_size;
        let height = asset.default_size[1] * scale_y.max(0.05) / tile_size;
        match asset.anchor {
            AnchorKind::TopLeft => SelectionMapBounds::from_min_max(x, y, x + width, y + height),
            AnchorKind::Center => SelectionMapBounds::from_min_max(
                x + 0.5 - width * 0.5,
                y + 0.5 - height * 0.5,
                x + 0.5 + width * 0.5,
                y + 0.5 + height * 0.5,
            ),
            AnchorKind::BottomCenter => SelectionMapBounds::from_min_max(
                x + 0.5 - width * 0.5,
                y + 1.0 - height,
                x + 0.5 + width * 0.5,
                y + 1.0,
            ),
        }
    }

    fn offset_selection_by_delta(
        &mut self,
        selection: &SelectedItem,
        delta: [f32; 2],
    ) -> Option<SelectedItem> {
        match selection.layer {
            LayerKind::Ground => {
                let [x, y] = parse_ground_selection_id(&selection.id)?;
                let updated = self.move_selected_item(
                    selection,
                    (x as f32 + delta[0]).round(),
                    (y as f32 + delta[1]).round(),
                )?;
                Some(SelectedItem {
                    layer: LayerKind::Ground,
                    id: ground_selection_id(updated[0], updated[1]),
                })
            }
            LayerKind::Decals | LayerKind::Objects | LayerKind::Entities => {
                let [x, y] = self.object_like_position(selection)?;
                self.move_selected_item(
                    selection,
                    (x + delta[0]).clamp(0.0, self.document.width as f32),
                    (y + delta[1]).clamp(0.0, self.document.height as f32),
                );
                Some(selection.clone())
            }
            LayerKind::Zones => {
                let max_x = self.document.width as f32;
                let max_y = self.document.height as f32;
                let zone = self
                    .document
                    .layers
                    .zones
                    .iter_mut()
                    .find(|zone| zone.id == selection.id)?;
                for point in &mut zone.points {
                    point[0] = (point[0] + delta[0]).clamp(0.0, max_x);
                    point[1] = (point[1] + delta[1]).clamp(0.0, max_y);
                }
                Some(selection.clone())
            }
            LayerKind::Collision => None,
        }
    }

    fn object_like_position(&self, selection: &SelectedItem) -> Option<[f32; 2]> {
        match selection.layer {
            LayerKind::Decals => self
                .document
                .layers
                .decals
                .iter()
                .find(|instance| instance.id == selection.id)
                .map(|instance| [instance.x, instance.y]),
            LayerKind::Objects => self
                .document
                .layers
                .objects
                .iter()
                .find(|instance| instance.id == selection.id)
                .map(|instance| [instance.x, instance.y]),
            LayerKind::Entities => self
                .document
                .layers
                .entities
                .iter()
                .find(|instance| instance.id == selection.id)
                .map(|instance| [instance.x, instance.y]),
            LayerKind::Ground | LayerKind::Zones | LayerKind::Collision => None,
        }
    }

    pub(crate) fn selections_in_screen_rect(
        &self,
        canvas_rect: Rect,
        selection_rect: Rect,
    ) -> Vec<SelectedItem> {
        let mut selections = Vec::new();
        for layer in LayerKind::ALL {
            if !self.layer_state(layer).visible {
                continue;
            }
            match layer {
                LayerKind::Ground => {
                    for tile in &self.document.layers.ground {
                        let selection = SelectedItem {
                            layer,
                            id: ground_selection_id(tile.x, tile.y),
                        };
                        if self
                            .selection_screen_rect(canvas_rect, &selection)
                            .is_some_and(|rect| rect.intersects(selection_rect))
                        {
                            selections.push(selection);
                        }
                    }
                }
                LayerKind::Decals => {
                    for instance in &self.document.layers.decals {
                        let selection = SelectedItem {
                            layer,
                            id: instance.id.clone(),
                        };
                        if self
                            .selection_screen_rect(canvas_rect, &selection)
                            .is_some_and(|rect| rect.intersects(selection_rect))
                        {
                            selections.push(selection);
                        }
                    }
                }
                LayerKind::Objects => {
                    for instance in &self.document.layers.objects {
                        let selection = SelectedItem {
                            layer,
                            id: instance.id.clone(),
                        };
                        if self
                            .selection_screen_rect(canvas_rect, &selection)
                            .is_some_and(|rect| rect.intersects(selection_rect))
                        {
                            selections.push(selection);
                        }
                    }
                }
                LayerKind::Entities => {
                    for instance in &self.document.layers.entities {
                        let selection = SelectedItem {
                            layer,
                            id: instance.id.clone(),
                        };
                        if self
                            .selection_screen_rect(canvas_rect, &selection)
                            .is_some_and(|rect| rect.intersects(selection_rect))
                        {
                            selections.push(selection);
                        }
                    }
                }
                LayerKind::Zones => {
                    for zone in &self.document.layers.zones {
                        let selection = SelectedItem {
                            layer,
                            id: zone.id.clone(),
                        };
                        if self
                            .selection_screen_rect(canvas_rect, &selection)
                            .is_some_and(|rect| rect.intersects(selection_rect))
                        {
                            selections.push(selection);
                        }
                    }
                }
                LayerKind::Collision => {}
            }
        }
        selections
    }

    pub(crate) fn hit_test_placed_item(
        &self,
        canvas_rect: Rect,
        pointer_pos: Pos2,
    ) -> Option<SelectedItem> {
        for entity in self.document.layers.entities.iter().rev() {
            if self
                .entity_instance_screen_rect(canvas_rect, entity)
                .is_some_and(|rect| rect.contains(pointer_pos))
            {
                return Some(SelectedItem {
                    layer: LayerKind::Entities,
                    id: entity.id.clone(),
                });
            }
        }

        for object in self.document.layers.objects.iter().rev() {
            if self
                .object_instance_screen_rect(canvas_rect, object)
                .is_some_and(|rect| rect.contains(pointer_pos))
            {
                return Some(SelectedItem {
                    layer: LayerKind::Objects,
                    id: object.id.clone(),
                });
            }
        }

        for decal in self.document.layers.decals.iter().rev() {
            if self
                .object_instance_screen_rect(canvas_rect, decal)
                .is_some_and(|rect| rect.contains(pointer_pos))
            {
                return Some(SelectedItem {
                    layer: LayerKind::Decals,
                    id: decal.id.clone(),
                });
            }
        }

        for zone in self.document.layers.zones.iter().rev() {
            if self
                .zone_screen_rect(canvas_rect, zone)
                .is_some_and(|rect| rect.contains(pointer_pos))
            {
                return Some(SelectedItem {
                    layer: LayerKind::Zones,
                    id: zone.id.clone(),
                });
            }
        }

        for tile in self.document.layers.ground.iter().rev() {
            if self
                .tile_screen_rect(canvas_rect, tile.x, tile.y, tile.w, tile.h)
                .contains(pointer_pos)
            {
                return Some(SelectedItem {
                    layer: LayerKind::Ground,
                    id: ground_selection_id(tile.x, tile.y),
                });
            }
        }

        None
    }

    pub(crate) fn move_selected_item(
        &mut self,
        selection: &SelectedItem,
        x: f32,
        y: f32,
    ) -> Option<[i32; 2]> {
        match selection.layer {
            LayerKind::Ground => {
                let Some([old_x, old_y]) = parse_ground_selection_id(&selection.id) else {
                    return None;
                };
                let Some(index) = self
                    .document
                    .layers
                    .ground
                    .iter()
                    .position(|tile| tile.x == old_x && tile.y == old_y)
                else {
                    return None;
                };
                let asset = self.document.layers.ground[index].asset.clone();
                let width = self.document.layers.ground[index].w.max(1);
                let height = self.document.layers.ground[index].h.max(1);
                let new_x = (x as i32).clamp(0, (self.document.width as i32 - width).max(0));
                let new_y = (y as i32).clamp(0, (self.document.height as i32 - height).max(0));
                let flip_x = self.document.layers.ground[index].flip_x;
                let rotation = self.document.layers.ground[index].rotation;
                self.document.layers.ground.remove(index);
                self.document.place_tile(&asset, new_x, new_y);
                if let Some(tile) = self
                    .document
                    .layers
                    .ground
                    .iter_mut()
                    .find(|tile| tile.x == new_x && tile.y == new_y)
                {
                    tile.w = width;
                    tile.h = height;
                    tile.flip_x = flip_x;
                    tile.rotation = rotation;
                }
                Some([new_x, new_y])
            }
            LayerKind::Decals => {
                if let Some(instance) = self
                    .document
                    .layers
                    .decals
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.x = x;
                    instance.y = y;
                }
                None
            }
            LayerKind::Objects => {
                if let Some(instance) = self
                    .document
                    .layers
                    .objects
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.x = x;
                    instance.y = y;
                }
                None
            }
            LayerKind::Entities => {
                if let Some(instance) = self
                    .document
                    .layers
                    .entities
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.x = x;
                    instance.y = y;
                }
                None
            }
            LayerKind::Zones | LayerKind::Collision => None,
        }
    }

    pub(crate) fn delete_selected_item(&mut self, selection: &SelectedItem) {
        match selection.layer {
            LayerKind::Ground => {
                if let Some([x, y]) = parse_ground_selection_id(&selection.id) {
                    self.document
                        .layers
                        .ground
                        .retain(|tile| tile.x != x || tile.y != y);
                }
            }
            LayerKind::Decals => self
                .document
                .layers
                .decals
                .retain(|instance| instance.id != selection.id),
            LayerKind::Objects => self
                .document
                .layers
                .objects
                .retain(|instance| instance.id != selection.id),
            LayerKind::Entities => self
                .document
                .layers
                .entities
                .retain(|instance| instance.id != selection.id),
            LayerKind::Zones => self
                .document
                .layers
                .zones
                .retain(|instance| instance.id != selection.id),
            LayerKind::Collision => {}
        }
    }

    pub(crate) fn flip_selected_item(&mut self) {
        let Some(selection) = self.selected_item.clone() else {
            self.status = "Select an item before flipping".to_owned();
            return;
        };
        let Some((flip_x, rotation)) = self.transform_for_selection(&selection) else {
            self.status = format!("No transform target for {}", selection.label());
            return;
        };

        if self.layer_state(selection.layer).locked {
            self.status = format!("{} 已锁定", selection.layer.zh_label());
            return;
        }
        self.push_undo_snapshot();
        self.set_transform_for_selection(&selection, !flip_x, rotation);
        self.mark_dirty();
        self.status = format!("Flipped {}", selection.label());
    }

    pub(crate) fn rotate_selected_item(&mut self, delta: i32) {
        let Some(selection) = self.selected_item.clone() else {
            self.status = "Select an item before rotating".to_owned();
            return;
        };
        let Some((flip_x, rotation)) = self.transform_for_selection(&selection) else {
            self.status = format!("No transform target for {}", selection.label());
            return;
        };

        if self.layer_state(selection.layer).locked {
            self.status = format!("{} 已锁定", selection.layer.zh_label());
            return;
        }
        self.push_undo_snapshot();
        self.set_transform_for_selection(&selection, flip_x, normalize_rotation(rotation + delta));
        self.mark_dirty();
        self.status = format!("Rotated {}", selection.label());
    }

    pub(crate) fn reset_selected_transform(&mut self) {
        let Some(selection) = self.selected_item.clone() else {
            self.status = "Select an item before resetting transform".to_owned();
            return;
        };

        if self.layer_state(selection.layer).locked {
            self.status = format!("{} 已锁定", selection.layer.zh_label());
            return;
        }
        self.push_undo_snapshot();
        self.set_transform_for_selection(&selection, false, 0);
        self.mark_dirty();
        self.status = format!("Reset transform for {}", selection.label());
    }

    pub(crate) fn transform_for_selection(&self, selection: &SelectedItem) -> Option<(bool, i32)> {
        match selection.layer {
            LayerKind::Ground => {
                let [x, y] = parse_ground_selection_id(&selection.id)?;
                self.document
                    .layers
                    .ground
                    .iter()
                    .find(|tile| tile.x == x && tile.y == y)
                    .map(|tile| (tile.flip_x, tile.rotation))
            }
            LayerKind::Decals => self
                .document
                .layers
                .decals
                .iter()
                .find(|instance| instance.id == selection.id)
                .map(|instance| (instance.flip_x, instance.rotation)),
            LayerKind::Objects => self
                .document
                .layers
                .objects
                .iter()
                .find(|instance| instance.id == selection.id)
                .map(|instance| (instance.flip_x, instance.rotation)),
            LayerKind::Entities => self
                .document
                .layers
                .entities
                .iter()
                .find(|instance| instance.id == selection.id)
                .map(|instance| (instance.flip_x, instance.rotation)),
            LayerKind::Zones | LayerKind::Collision => None,
        }
    }

    pub(crate) fn set_transform_for_selection(
        &mut self,
        selection: &SelectedItem,
        flip_x: bool,
        rotation: i32,
    ) {
        let rotation = normalize_rotation(rotation);
        match selection.layer {
            LayerKind::Ground => {
                if let Some([x, y]) = parse_ground_selection_id(&selection.id) {
                    if let Some(tile) = self
                        .document
                        .layers
                        .ground
                        .iter_mut()
                        .find(|tile| tile.x == x && tile.y == y)
                    {
                        tile.flip_x = flip_x;
                        tile.rotation = rotation;
                    }
                }
            }
            LayerKind::Decals => {
                if let Some(instance) = self
                    .document
                    .layers
                    .decals
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.flip_x = flip_x;
                    instance.rotation = rotation;
                }
            }
            LayerKind::Objects => {
                if let Some(instance) = self
                    .document
                    .layers
                    .objects
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.flip_x = flip_x;
                    instance.rotation = rotation;
                }
            }
            LayerKind::Entities => {
                if let Some(instance) = self
                    .document
                    .layers
                    .entities
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.flip_x = flip_x;
                    instance.rotation = rotation;
                }
            }
            LayerKind::Zones | LayerKind::Collision => {}
        }
    }

    pub(crate) fn set_z_index_for_selection(&mut self, selection: &SelectedItem, z_index: i32) {
        if self.layer_state(selection.layer).locked {
            return;
        }
        match selection.layer {
            LayerKind::Decals => {
                if let Some(instance) = self
                    .document
                    .layers
                    .decals
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.z_index = z_index;
                }
            }
            LayerKind::Objects => {
                if let Some(instance) = self
                    .document
                    .layers
                    .objects
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.z_index = z_index;
                }
            }
            LayerKind::Entities => {
                if let Some(instance) = self
                    .document
                    .layers
                    .entities
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    instance.z_index = z_index;
                }
            }
            LayerKind::Ground | LayerKind::Zones | LayerKind::Collision => {}
        }
    }

    pub(crate) fn ground_size_for_selection(&self) -> Option<[i32; 2]> {
        if self.current_selection_list().len() != 1 {
            return None;
        }
        let selection = self.selected_item.as_ref()?;
        if selection.layer != LayerKind::Ground {
            return None;
        }

        self.ground_size_for_selection_id(&selection.id)
    }

    pub(crate) fn ground_size_for_selection_id(&self, id: &str) -> Option<[i32; 2]> {
        let [x, y] = parse_ground_selection_id(id)?;
        self.document
            .layers
            .ground
            .iter()
            .find(|tile| tile.x == x && tile.y == y)
            .map(|tile| [tile.w.max(1), tile.h.max(1)])
    }

    pub(crate) fn set_ground_size_for_selection(&mut self, width: i32, height: i32) {
        let Some(selection) = self.selected_item.clone() else {
            return;
        };
        if selection.layer != LayerKind::Ground {
            return;
        }
        let Some([x, y]) = parse_ground_selection_id(&selection.id) else {
            return;
        };

        let max_width = (self.document.width as i32 - x).max(1);
        let max_height = (self.document.height as i32 - y).max(1);
        let mut resized_to = None;

        if self.layer_state(LayerKind::Ground).locked {
            self.status = "地表图层已锁定".to_owned();
            return;
        }
        self.push_undo_snapshot();
        if let Some(tile) = self
            .document
            .layers
            .ground
            .iter_mut()
            .find(|tile| tile.x == x && tile.y == y)
        {
            tile.w = width.clamp(1, max_width);
            tile.h = height.clamp(1, max_height);
            resized_to = Some([tile.w, tile.h]);
        }

        if let Some([width, height]) = resized_to {
            self.mark_dirty();
            self.status = format!("Resized Ground:{x},{y} to {width} x {height}");
        }
    }

    pub(crate) fn clamped_ground_footprint_at(&self, x: i32, y: i32) -> [i32; 2] {
        let max_width = (self.document.width as i32 - x).max(1);
        let max_height = (self.document.height as i32 - y).max(1);

        [
            self.ground_footprint_w.clamp(1, max_width),
            self.ground_footprint_h.clamp(1, max_height),
        ]
    }

    pub(crate) fn clamped_collision_brush_at(&self, x: f32, y: f32) -> [f32; 2] {
        let max_width = (self.document.width as f32 - x).max(0.125);
        let max_height = (self.document.height as f32 - y).max(0.125);

        [
            self.collision_brush_w.clamp(0.125, max_width),
            self.collision_brush_h.clamp(0.125, max_height),
        ]
    }

    pub(crate) fn selected_ground_footprint_at(&self, x: i32, y: i32) -> [i32; 2] {
        self.selected_asset()
            .map(|asset| self.clamped_tile_footprint_at(asset, x, y))
            .unwrap_or_else(|| self.clamped_ground_footprint_at(x, y))
    }

    pub(crate) fn clamped_tile_footprint_at(&self, asset: &AssetEntry, x: i32, y: i32) -> [i32; 2] {
        let [width, height] = self.asset_tile_footprint(asset);
        let max_width = (self.document.width as i32 - x).max(1);
        let max_height = (self.document.height as i32 - y).max(1);
        [width.clamp(1, max_width), height.clamp(1, max_height)]
    }

    pub(crate) fn asset_tile_footprint(&self, asset: &AssetEntry) -> [i32; 2] {
        asset
            .footprint
            .or_else(|| infer_tile_footprint(asset.default_size, self.document.tile_size))
            .unwrap_or([1, 1])
    }

    pub(crate) fn draw_layers(&self, canvas_rect: Rect, painter: &egui::Painter) {
        let visible_tiles = self.visible_tile_bounds(canvas_rect);
        if self.layer_state(LayerKind::Ground).visible {
            for tile in &self.document.layers.ground {
                if !tile_intersects_rect(
                    tile,
                    visible_tiles.min_x,
                    visible_tiles.min_y,
                    visible_tiles.max_x,
                    visible_tiles.max_y,
                ) {
                    continue;
                }
                let rect = self.tile_screen_rect(canvas_rect, tile.x, tile.y, tile.w, tile.h);
                self.draw_asset_image(painter, &tile.asset, rect, tile.flip_x, tile.rotation);
            }
        }

        if self.layer_state(LayerKind::Decals).visible {
            for decal in &self.document.layers.decals {
                let Some(rect) = self.object_instance_screen_rect(canvas_rect, decal) else {
                    continue;
                };
                if !rect.intersects(canvas_rect) {
                    continue;
                }
                self.draw_object_like_rect(
                    painter,
                    &decal.asset,
                    rect,
                    decal.flip_x,
                    decal.rotation,
                );
            }
        }

        if self.layer_state(LayerKind::Objects).visible {
            let mut objects = self
                .document
                .layers
                .objects
                .iter()
                .filter_map(|object| {
                    self.object_instance_screen_rect(canvas_rect, object)
                        .filter(|rect| rect.intersects(canvas_rect))
                        .map(|rect| (object, rect))
                })
                .collect::<Vec<_>>();
            objects.sort_by(|left, right| {
                left.0
                    .z_index
                    .cmp(&right.0.z_index)
                    .then_with(|| left.0.y.total_cmp(&right.0.y))
            });
            for (object, rect) in objects {
                self.draw_object_like_rect(
                    painter,
                    &object.asset,
                    rect,
                    object.flip_x,
                    object.rotation,
                );
            }
        }

        if self.layer_state(LayerKind::Entities).visible {
            let mut entities = self
                .document
                .layers
                .entities
                .iter()
                .filter_map(|entity| {
                    self.entity_instance_screen_rect(canvas_rect, entity)
                        .filter(|rect| rect.intersects(canvas_rect))
                        .map(|rect| (entity, rect))
                })
                .collect::<Vec<_>>();
            entities.sort_by(|left, right| {
                left.0
                    .z_index
                    .cmp(&right.0.z_index)
                    .then_with(|| left.0.y.total_cmp(&right.0.y))
            });
            for (entity, rect) in entities {
                self.draw_object_like_rect(
                    painter,
                    &entity.asset,
                    rect,
                    entity.flip_x,
                    entity.rotation,
                );
            }
        }

        if self.show_zones && self.layer_state(LayerKind::Zones).visible {
            self.draw_zones(canvas_rect, painter);
        }
    }

    pub(crate) fn draw_zones(&self, canvas_rect: Rect, painter: &egui::Painter) {
        let tile_size = self.document.tile_size as f32;
        for zone in &self.document.layers.zones {
            if !self
                .zone_screen_rect(canvas_rect, zone)
                .is_some_and(|rect| rect.intersects(canvas_rect))
            {
                continue;
            }
            let (stroke_color, fill_color) = zone_colors(&zone.zone_type);
            let points = zone
                .points
                .iter()
                .map(|point| {
                    self.world_to_screen(
                        canvas_rect,
                        vec2(point[0] * tile_size, point[1] * tile_size),
                    )
                })
                .collect::<Vec<_>>();
            if points.len() < 2 {
                continue;
            }
            if zone.zone_type == "CollisionLine" {
                for pair in points.windows(2) {
                    painter.line_segment([pair[0], pair[1]], Stroke::new(3.0, stroke_color));
                }
            } else {
                painter.add(Shape::convex_polygon(
                    points.clone(),
                    fill_color,
                    Stroke::new(1.5, stroke_color),
                ));
            }
            if self.show_zone_labels {
                let center = polygon_screen_center(&points);
                painter.text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    format!("{}\\n{}", zone.id, zone.zone_type),
                    egui::TextStyle::Small.resolve(&egui::Style::default()),
                    THEME_TEXT,
                );
            }
        }
    }

    fn draw_object_like_rect(
        &self,
        painter: &egui::Painter,
        asset_id: &str,
        rect: Rect,
        flip_x: bool,
        rotation: i32,
    ) {
        self.draw_asset_image_tinted(painter, asset_id, rect, flip_x, rotation, Color32::WHITE);
    }

    pub(crate) fn object_screen_rect_scaled(
        &self,
        canvas_rect: Rect,
        asset_id: &str,
        x: f32,
        y: f32,
        scale_x: f32,
        scale_y: f32,
    ) -> Option<Rect> {
        let asset = self.registry.get(asset_id)?;
        let tile_size = self.document.tile_size as f32;
        let anchor = self.world_to_screen(
            canvas_rect,
            anchor_grid_to_world(tile_size, x, y, asset.anchor),
        );
        let size = vec2(
            asset.default_size[0] * scale_x.max(0.05),
            asset.default_size[1] * scale_y.max(0.05),
        ) * self.zoom;

        Some(screen_rect_from_anchor(anchor, size, asset.anchor))
    }

    pub(crate) fn object_instance_screen_rect(
        &self,
        canvas_rect: Rect,
        instance: &content::ObjectInstance,
    ) -> Option<Rect> {
        self.object_screen_rect_scaled(
            canvas_rect,
            &instance.asset,
            instance.x,
            instance.y,
            instance.scale_x,
            instance.scale_y,
        )
    }

    pub(crate) fn entity_instance_screen_rect(
        &self,
        canvas_rect: Rect,
        instance: &content::EntityInstance,
    ) -> Option<Rect> {
        self.object_screen_rect_scaled(
            canvas_rect,
            &instance.asset,
            instance.x,
            instance.y,
            instance.scale_x,
            instance.scale_y,
        )
    }

    pub(crate) fn zone_screen_rect(
        &self,
        canvas_rect: Rect,
        zone: &content::ZoneInstance,
    ) -> Option<Rect> {
        let tile_size = self.document.tile_size as f32;
        let mut points = zone.points.iter().map(|point| {
            self.world_to_screen(
                canvas_rect,
                vec2(point[0] * tile_size, point[1] * tile_size),
            )
        });
        let first = points.next()?;
        let mut min = first;
        let mut max = first;
        for point in points {
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.y);
            max.x = max.x.max(point.x);
            max.y = max.y.max(point.y);
        }
        Some(Rect::from_min_max(min, max))
    }

    pub(crate) fn draw_asset_image(
        &self,
        painter: &egui::Painter,
        asset_id: &str,
        rect: Rect,
        flip_x: bool,
        rotation: i32,
    ) {
        self.draw_asset_image_tinted(painter, asset_id, rect, flip_x, rotation, Color32::WHITE);
    }

    pub(crate) fn draw_asset_image_tinted(
        &self,
        painter: &egui::Painter,
        asset_id: &str,
        rect: Rect,
        flip_x: bool,
        rotation: i32,
        tint: Color32,
    ) {
        if let Some(texture) = self.thumbnails.get(asset_id) {
            let image_rect = fit_centered_rect(rect, texture.size_vec2());
            paint_transformed_image(painter, texture.id(), image_rect, flip_x, rotation, tint);
        } else {
            painter.rect_filled(
                rect,
                1.0,
                Color32::from_rgba_unmultiplied(68, 72, 64, tint.a()),
            );
        }
    }

    pub(crate) fn tile_screen_rect(
        &self,
        canvas_rect: Rect,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> Rect {
        let tile_size = self.document.tile_size as f32;
        let world = vec2(x as f32 * tile_size, y as f32 * tile_size);
        let size = vec2(w.max(1) as f32 * tile_size, h.max(1) as f32 * tile_size);
        Rect::from_min_size(self.world_to_screen(canvas_rect, world), size * self.zoom)
    }

    pub(crate) fn map_unit_screen_rect(
        &self,
        canvas_rect: Rect,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) -> Rect {
        let tile_size = self.document.tile_size as f32;
        let world = vec2(x * tile_size, y * tile_size);
        let size = vec2(w.max(0.05) * tile_size, h.max(0.05) * tile_size);
        Rect::from_min_size(self.world_to_screen(canvas_rect, world), size * self.zoom)
    }

    pub(crate) fn draw_collision(&self, canvas_rect: Rect, painter: &egui::Painter) {
        if !self.layer_state(LayerKind::Collision).visible {
            return;
        }
        let tile_size = self.document.tile_size as f32;
        let visible_tiles = self.visible_tile_bounds(canvas_rect);
        for cell in &self.document.layers.collision {
            if !cell.solid {
                continue;
            }

            let bounds = cell.bounds();
            if !bounds_intersects_tile_rect(bounds.x, bounds.y, bounds.w, bounds.h, visible_tiles) {
                continue;
            }
            let world = vec2(bounds.x * tile_size, bounds.y * tile_size);
            let rect = Rect::from_min_size(
                self.world_to_screen(canvas_rect, world),
                vec2(bounds.w * tile_size, bounds.h * tile_size) * self.zoom,
            );
            painter.rect_filled(
                rect,
                0.0,
                Color32::from_rgba_unmultiplied(
                    THEME_COLLISION.r(),
                    THEME_COLLISION.g(),
                    THEME_COLLISION.b(),
                    86,
                ),
            );
        }

        for tile in &self.document.layers.ground {
            if !tile_intersects_rect(
                tile,
                visible_tiles.min_x,
                visible_tiles.min_y,
                visible_tiles.max_x,
                visible_tiles.max_y,
            ) {
                continue;
            }
            let rect = self
                .registry
                .get(&tile.asset)
                .and_then(|asset| asset.default_collision_rect);
            if let Some(rect) = rect {
                self.draw_instance_collision_rect(
                    canvas_rect,
                    painter,
                    tile.x as f32,
                    tile.y as f32,
                    rect,
                );
            }
        }

        for instance in &self.document.layers.objects {
            if !self
                .object_instance_screen_rect(canvas_rect, instance)
                .is_some_and(|rect| rect.intersects(canvas_rect))
            {
                continue;
            }
            let rect = instance.collision_rect.or_else(|| {
                self.registry
                    .get(&instance.asset)
                    .and_then(|asset| asset.default_collision_rect)
            });
            if let Some(rect) = rect {
                self.draw_instance_collision_rect(
                    canvas_rect,
                    painter,
                    instance.x,
                    instance.y,
                    rect,
                );
            }
        }

        for entity in &self.document.layers.entities {
            if !self
                .entity_instance_screen_rect(canvas_rect, entity)
                .is_some_and(|rect| rect.intersects(canvas_rect))
            {
                continue;
            }
            let rect = entity.collision_rect.or_else(|| {
                self.registry
                    .get(&entity.asset)
                    .and_then(|asset| asset.default_collision_rect)
            });
            if let Some(rect) = rect {
                self.draw_instance_collision_rect(canvas_rect, painter, entity.x, entity.y, rect);
            }
        }
    }

    fn draw_instance_collision_rect(
        &self,
        canvas_rect: Rect,
        painter: &egui::Painter,
        x: f32,
        y: f32,
        rect: content::InstanceRect,
    ) {
        let rect = self.map_unit_screen_rect(
            canvas_rect,
            x + rect.offset[0],
            y + rect.offset[1],
            rect.size[0],
            rect.size[1],
        );
        if !rect.intersects(canvas_rect) {
            return;
        }
        painter.rect_filled(
            rect,
            0.0,
            Color32::from_rgba_unmultiplied(
                THEME_COLLISION.r(),
                THEME_COLLISION.g(),
                THEME_COLLISION.b(),
                58,
            ),
        );
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, THEME_COLLISION),
            StrokeKind::Inside,
        );
    }

    pub(crate) fn draw_entity_bounds(&self, canvas_rect: Rect, painter: &egui::Painter) {
        let tile_size = self.document.tile_size as f32;
        let visible_tiles = self.visible_tile_bounds(canvas_rect);
        for entity in &self.document.layers.entities {
            if !bounds_intersects_tile_rect(entity.x, entity.y, 1.0, 1.0, visible_tiles) {
                continue;
            }
            let world = vec2(entity.x * tile_size, entity.y * tile_size);
            let rect = Rect::from_min_size(
                self.world_to_screen(canvas_rect, world),
                vec2(tile_size, tile_size) * self.zoom,
            );
            if !rect.intersects(canvas_rect) {
                continue;
            }
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(1.5, THEME_ACCENT),
                StrokeKind::Inside,
            );
        }
    }

    pub(crate) fn draw_selection_bounds(&self, canvas_rect: Rect, painter: &egui::Painter) {
        let selections = self.current_selection_list();
        if selections.is_empty() {
            return;
        }

        let mut group_rect: Option<Rect> = None;
        for (index, selection) in selections.iter().enumerate() {
            let Some(rect) = self.selection_screen_rect(canvas_rect, selection) else {
                continue;
            };
            group_rect = Some(match group_rect {
                Some(group) => group.union(rect),
                None => rect,
            });
            let color = if index == 0 {
                THEME_SELECTION
            } else {
                THEME_MULTI_SELECTION
            };
            painter.rect_stroke(
                rect.expand(3.0),
                2.0,
                Stroke::new(2.0, color),
                StrokeKind::Inside,
            );
        }

        if selections.len() == 1 {
            let selection = &selections[0];
            let Some(rect) = self.selection_screen_rect(canvas_rect, selection) else {
                return;
            };
            if matches!(
                selection.layer,
                LayerKind::Decals | LayerKind::Objects | LayerKind::Entities
            ) && !self.layer_state(selection.layer).locked
            {
                for handle in resize_handle_rects(rect) {
                    painter.rect_filled(handle, 1.5, THEME_SELECTION);
                    painter.rect_stroke(
                        handle,
                        1.5,
                        Stroke::new(1.0, THEME_WARNING_BG),
                        StrokeKind::Inside,
                    );
                }
            }

            if selection.layer == LayerKind::Zones {
                self.draw_zone_vertex_handles(canvas_rect, selection, painter);
            }
        } else if let Some(rect) = group_rect {
            painter.rect_stroke(
                rect.expand(8.0),
                2.0,
                Stroke::new(1.5, THEME_MULTI_SELECTION),
                StrokeKind::Inside,
            );
        }
    }

    pub(crate) fn draw_selection_marquee(&self, painter: &egui::Painter) {
        let Some(marquee) = &self.selection_marquee else {
            return;
        };
        let rect = Rect::from_two_pos(marquee.start, marquee.current);
        painter.rect_filled(
            rect,
            0.0,
            Color32::from_rgba_unmultiplied(
                THEME_MULTI_SELECTION.r(),
                THEME_MULTI_SELECTION.g(),
                THEME_MULTI_SELECTION.b(),
                34,
            ),
        );
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.5, THEME_MULTI_SELECTION),
            StrokeKind::Inside,
        );
    }

    pub(crate) fn draw_zone_vertex_handles(
        &self,
        canvas_rect: Rect,
        selection: &SelectedItem,
        painter: &egui::Painter,
    ) {
        let Some(zone) = self
            .document
            .layers
            .zones
            .iter()
            .find(|zone| zone.id == selection.id)
        else {
            return;
        };
        let tile_size = self.document.tile_size as f32;
        for (index, point) in zone.points.iter().enumerate() {
            let screen = self.world_to_screen(
                canvas_rect,
                vec2(point[0] * tile_size, point[1] * tile_size),
            );
            let rect = Rect::from_center_size(screen, vec2(9.0, 9.0));
            painter.rect_filled(rect, 2.0, THEME_SELECTION);
            painter.rect_stroke(
                rect,
                2.0,
                Stroke::new(1.0, THEME_WARNING_BG),
                StrokeKind::Inside,
            );
            painter.text(
                screen + vec2(8.0, -8.0),
                egui::Align2::LEFT_CENTER,
                index.to_string(),
                egui::TextStyle::Small.resolve(&egui::Style::default()),
                THEME_TEXT,
            );
        }
    }

    pub(crate) fn selection_screen_rect(
        &self,
        canvas_rect: Rect,
        selection: &SelectedItem,
    ) -> Option<Rect> {
        match selection.layer {
            LayerKind::Ground => parse_ground_selection_id(&selection.id).and_then(|[x, y]| {
                self.document
                    .layers
                    .ground
                    .iter()
                    .find(|tile| tile.x == x && tile.y == y)
                    .map(|tile| self.tile_screen_rect(canvas_rect, x, y, tile.w, tile.h))
            }),
            LayerKind::Decals => self
                .document
                .layers
                .decals
                .iter()
                .find(|instance| instance.id == selection.id)
                .and_then(|instance| self.object_instance_screen_rect(canvas_rect, instance)),
            LayerKind::Objects => self
                .document
                .layers
                .objects
                .iter()
                .find(|instance| instance.id == selection.id)
                .and_then(|instance| self.object_instance_screen_rect(canvas_rect, instance)),
            LayerKind::Entities => self
                .document
                .layers
                .entities
                .iter()
                .find(|instance| instance.id == selection.id)
                .and_then(|instance| self.entity_instance_screen_rect(canvas_rect, instance)),
            LayerKind::Zones => self
                .document
                .layers
                .zones
                .iter()
                .find(|zone| zone.id == selection.id)
                .and_then(|zone| self.zone_screen_rect(canvas_rect, zone)),
            LayerKind::Collision => None,
        }
    }

    pub(crate) fn draw_brush_preview(
        &self,
        canvas_rect: Rect,
        painter: &egui::Painter,
        hover_pos: Option<Pos2>,
        modifiers: Modifiers,
    ) {
        if !matches!(
            self.tool,
            ToolKind::Brush | ToolKind::Rectangle | ToolKind::Erase | ToolKind::Collision
        ) {
            return;
        }

        if self.tool == ToolKind::Rectangle && self.rectangle_drag_start.is_some() {
            return;
        }

        let Some(hover_pos) = hover_pos else {
            return;
        };

        let preview_layer = if self.tool == ToolKind::Collision {
            LayerKind::Collision
        } else {
            self.active_layer
        };

        if preview_layer == LayerKind::Zones {
            return;
        };

        if preview_layer == LayerKind::Collision {
            let Some(raw) = self.screen_to_map_position(canvas_rect, hover_pos) else {
                self.draw_canvas_hover_label(painter, hover_pos, THEME_ERROR, "地图外");
                return;
            };
            let [x, y] = self.snapped_collision_position(raw, modifiers);
            self.draw_collision_brush_preview(canvas_rect, painter, x, y);
            return;
        }

        let Some([x, y]) = self.screen_to_tile(canvas_rect, hover_pos) else {
            self.draw_canvas_hover_label(painter, hover_pos, THEME_ERROR, "地图外");
            return;
        };

        match preview_layer {
            LayerKind::Ground | LayerKind::Collision => {
                self.draw_tile_brush_preview(canvas_rect, painter, preview_layer, x, y);
            }
            LayerKind::Decals | LayerKind::Objects | LayerKind::Entities => {
                if self.tool == ToolKind::Erase {
                    self.draw_object_erase_preview(canvas_rect, painter, preview_layer, x, y);
                } else {
                    self.draw_asset_placement_preview(
                        canvas_rect,
                        painter,
                        preview_layer,
                        hover_pos,
                        modifiers,
                    );
                }
            }
            LayerKind::Zones => {}
        }
    }

    pub(crate) fn draw_stamp_preview(
        &self,
        canvas_rect: Rect,
        painter: &egui::Painter,
        hover_pos: Option<Pos2>,
    ) {
        if self.tool != ToolKind::Stamp {
            return;
        }

        if let Some(drag) = &self.stamp_capture_drag {
            let [min_x, min_y, max_x, max_y] = normalized_tile_rect(drag.start, drag.current);
            let rect = self.tile_screen_rect(
                canvas_rect,
                min_x,
                min_y,
                max_x - min_x + 1,
                max_y - min_y + 1,
            );
            painter.rect_filled(
                rect,
                0.0,
                Color32::from_rgba_unmultiplied(
                    THEME_MULTI_SELECTION.r(),
                    THEME_MULTI_SELECTION.g(),
                    THEME_MULTI_SELECTION.b(),
                    38,
                ),
            );
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(2.0, THEME_MULTI_SELECTION),
                StrokeKind::Inside,
            );
            self.draw_canvas_hover_label(
                painter,
                rect.right_top(),
                THEME_MULTI_SELECTION,
                "生成 Stamp",
            );
            return;
        }

        let Some(hover_pos) = hover_pos else {
            return;
        };
        let Some([x, y]) = self.screen_to_tile(canvas_rect, hover_pos) else {
            self.draw_canvas_hover_label(painter, hover_pos, THEME_ERROR, "地图外");
            return;
        };
        let Some(pattern) = &self.stamp_pattern else {
            self.draw_canvas_hover_label(painter, hover_pos, THEME_WARNING, "拖拽框选生成 Stamp");
            return;
        };

        let warnings = self.stamp_warnings_at(pattern, x, y);
        let color = if warnings.is_empty() {
            THEME_ACCENT
        } else {
            THEME_WARNING
        };
        let tint = if warnings.is_empty() {
            Color32::from_rgba_unmultiplied(255, 255, 255, 138)
        } else {
            Color32::from_rgba_unmultiplied(255, 232, 190, 122)
        };

        for item in &pattern.items {
            match item {
                StampItem::Ground(tile) => {
                    let rect = self.tile_screen_rect(
                        canvas_rect,
                        x + tile.x,
                        y + tile.y,
                        tile.w.max(1),
                        tile.h.max(1),
                    );
                    self.draw_asset_image_tinted(
                        painter,
                        &tile.asset,
                        rect,
                        tile.flip_x,
                        tile.rotation,
                        tint,
                    );
                }
                StampItem::Decal(instance) | StampItem::Object(instance) => {
                    if let Some(rect) = self.object_screen_rect_scaled(
                        canvas_rect,
                        &instance.asset,
                        x as f32 + instance.x,
                        y as f32 + instance.y,
                        instance.scale_x,
                        instance.scale_y,
                    ) {
                        self.draw_asset_image_tinted(
                            painter,
                            &instance.asset,
                            rect,
                            instance.flip_x,
                            instance.rotation,
                            tint,
                        );
                    }
                }
                StampItem::Entity(instance) => {
                    if let Some(rect) = self.object_screen_rect_scaled(
                        canvas_rect,
                        &instance.asset,
                        x as f32 + instance.x,
                        y as f32 + instance.y,
                        instance.scale_x,
                        instance.scale_y,
                    ) {
                        self.draw_asset_image_tinted(
                            painter,
                            &instance.asset,
                            rect,
                            instance.flip_x,
                            instance.rotation,
                            tint,
                        );
                    }
                }
            }
        }

        let bounds = self.tile_screen_rect(canvas_rect, x, y, pattern.width, pattern.height);
        self.paint_preview_rect(painter, bounds, color, !warnings.is_empty());
        let label = if warnings.is_empty() {
            format!("Stamp {} 个对象", pattern.item_count())
        } else {
            warnings.join("\n")
        };
        self.draw_canvas_hover_label(painter, bounds.right_top(), color, &label);
    }

    fn stamp_warnings_at(&self, pattern: &StampPattern, x: i32, y: i32) -> Vec<String> {
        let mut warnings = Vec::new();
        for layer in [
            LayerKind::Ground,
            LayerKind::Decals,
            LayerKind::Objects,
            LayerKind::Entities,
        ] {
            if pattern.items.iter().any(|item| item.layer() == layer)
                && self.layer_state(layer).locked
            {
                warnings.push(format!("{} 已锁定", layer.zh_label()));
            }
        }
        if x + pattern.width > self.document.width as i32
            || y + pattern.height > self.document.height as i32
        {
            warnings.push("Stamp 超出地图边界，粘贴时会跳过越界对象".to_owned());
        }
        warnings
    }

    fn draw_tile_brush_preview(
        &self,
        canvas_rect: Rect,
        painter: &egui::Painter,
        layer: LayerKind,
        x: i32,
        y: i32,
    ) {
        let mut warnings = Vec::new();
        if self.layer_state(layer).locked {
            warnings.push(format!("{} 已锁定", layer.zh_label()));
        }

        let selected_asset = self.selected_asset();
        if layer == LayerKind::Ground && self.tool != ToolKind::Erase && selected_asset.is_none() {
            let rect = self.tile_screen_rect(canvas_rect, x, y, 1, 1);
            self.paint_preview_rect(painter, rect, THEME_WARNING, true);
            self.draw_canvas_hover_label(
                painter,
                rect.right_top(),
                THEME_WARNING,
                "先选择地表素材",
            );
            return;
        }

        if let Some(asset) = selected_asset.filter(|_| layer == LayerKind::Ground) {
            warnings.extend(self.asset_placement_warnings(asset, layer));
        }

        if layer == LayerKind::Collision {
            let desired = [
                self.collision_brush_w.max(0.125),
                self.collision_brush_h.max(0.125),
            ];
            let clamped = self.clamped_collision_brush_at(x as f32, y as f32);
            if desired != clamped {
                warnings.push(format!(
                    "边界裁切 {:.2}x{:.2} -> {:.2}x{:.2}",
                    desired[0], desired[1], clamped[0], clamped[1]
                ));
            }

            let rect =
                self.map_unit_screen_rect(canvas_rect, x as f32, y as f32, clamped[0], clamped[1]);
            let color = if self.layer_state(layer).locked {
                THEME_ERROR
            } else if !warnings.is_empty() {
                THEME_WARNING
            } else {
                THEME_COLLISION
            };
            self.paint_preview_rect(painter, rect, color, !warnings.is_empty());
            if !warnings.is_empty() {
                self.draw_canvas_hover_label(
                    painter,
                    rect.right_top(),
                    color,
                    &warnings.join("\n"),
                );
            }
            return;
        }

        let desired = match layer {
            LayerKind::Ground if self.tool == ToolKind::Erase => [
                self.ground_footprint_w.max(1),
                self.ground_footprint_h.max(1),
            ],
            LayerKind::Ground => selected_asset
                .map(|asset| self.asset_tile_footprint(asset))
                .unwrap_or([
                    self.ground_footprint_w.max(1),
                    self.ground_footprint_h.max(1),
                ]),
            _ => [1, 1],
        };
        let clamped = match layer {
            LayerKind::Ground if self.tool == ToolKind::Erase => {
                self.clamped_ground_footprint_at(x, y)
            }
            LayerKind::Ground => selected_asset
                .map(|asset| self.clamped_tile_footprint_at(asset, x, y))
                .unwrap_or_else(|| self.clamped_ground_footprint_at(x, y)),
            _ => [1, 1],
        };

        if desired != clamped {
            warnings.push(format!(
                "边界裁切 {}x{} -> {}x{}",
                desired[0], desired[1], clamped[0], clamped[1]
            ));
        }

        let [width, height] = clamped;
        let rect = self.tile_screen_rect(canvas_rect, x, y, width, height);
        let color = if self.layer_state(layer).locked {
            THEME_ERROR
        } else if !warnings.is_empty() {
            THEME_WARNING
        } else if layer == LayerKind::Collision {
            THEME_COLLISION
        } else {
            THEME_ACCENT
        };
        self.paint_preview_rect(painter, rect, color, !warnings.is_empty());

        if layer == LayerKind::Ground && self.tool != ToolKind::Erase {
            if let Some(asset) = selected_asset {
                self.draw_asset_image_tinted(
                    painter,
                    &asset.id,
                    rect,
                    false,
                    0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 150),
                );
                painter.rect_stroke(rect, 0.0, Stroke::new(2.0, color), StrokeKind::Inside);
            }
        }

        if !warnings.is_empty() {
            self.draw_canvas_hover_label(painter, rect.right_top(), color, &warnings.join("\n"));
        }
    }

    fn draw_collision_brush_preview(
        &self,
        canvas_rect: Rect,
        painter: &egui::Painter,
        x: f32,
        y: f32,
    ) {
        let mut warnings = Vec::new();
        if self.layer_state(LayerKind::Collision).locked {
            warnings.push("碰撞 已锁定".to_owned());
        }

        let desired = [
            self.collision_brush_w.max(0.125),
            self.collision_brush_h.max(0.125),
        ];
        let clamped = self.clamped_collision_brush_at(x, y);
        if desired != clamped {
            warnings.push(format!(
                "边界裁切 {:.2}x{:.2} -> {:.2}x{:.2}",
                desired[0], desired[1], clamped[0], clamped[1]
            ));
        }

        let rect = self.map_unit_screen_rect(canvas_rect, x, y, clamped[0], clamped[1]);
        let color = if self.layer_state(LayerKind::Collision).locked {
            THEME_ERROR
        } else if !warnings.is_empty() {
            THEME_WARNING
        } else {
            THEME_COLLISION
        };
        self.paint_preview_rect(painter, rect, color, !warnings.is_empty());
        if !warnings.is_empty() {
            self.draw_canvas_hover_label(painter, rect.right_top(), color, &warnings.join("\n"));
        }
    }

    fn draw_asset_placement_preview(
        &self,
        canvas_rect: Rect,
        painter: &egui::Painter,
        layer: LayerKind,
        hover_pos: Pos2,
        modifiers: Modifiers,
    ) {
        let Some(asset) = self.selected_asset() else {
            self.draw_canvas_hover_label(painter, hover_pos, THEME_WARNING, "先选择素材");
            return;
        };
        let Some(raw_pos) = self.screen_to_map_position(canvas_rect, hover_pos) else {
            return;
        };
        let place_pos = self.snapped_map_position(raw_pos, Some(asset), modifiers);
        let Some(rect) = self.object_screen_rect_scaled(
            canvas_rect,
            &asset.id,
            place_pos[0],
            place_pos[1],
            1.0,
            1.0,
        ) else {
            return;
        };

        let mut warnings = self.asset_placement_warnings(asset, layer);
        let map_rect = self.map_screen_rect(canvas_rect);
        if !map_rect.contains(rect.min) || !map_rect.contains(rect.max) {
            warnings.push("图像超出地图边界".to_owned());
        }

        let color = if self.layer_state(layer).locked {
            THEME_ERROR
        } else if !warnings.is_empty() {
            THEME_WARNING
        } else {
            THEME_ACCENT
        };
        let tile_x = place_pos[0].floor() as i32;
        let tile_y = place_pos[1].floor() as i32;
        if let Some([width, height]) = self.asset_preview_footprint(asset) {
            let footprint = self.tile_screen_rect(canvas_rect, tile_x, tile_y, width, height);
            self.paint_preview_rect(painter, footprint, color, !warnings.is_empty());
        } else {
            let anchor_cell = self.tile_screen_rect(canvas_rect, tile_x, tile_y, 1, 1);
            painter.rect_stroke(
                anchor_cell,
                0.0,
                Stroke::new(
                    1.0,
                    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 150),
                ),
                StrokeKind::Inside,
            );
        }

        let tint = if warnings.is_empty() {
            Color32::from_rgba_unmultiplied(255, 255, 255, 155)
        } else {
            Color32::from_rgba_unmultiplied(255, 232, 190, 140)
        };
        self.draw_asset_image_tinted(painter, &asset.id, rect, false, 0, tint);
        painter.rect_stroke(
            rect.expand(2.0),
            2.0,
            Stroke::new(2.0, color),
            StrokeKind::Inside,
        );

        if !warnings.is_empty() {
            self.draw_canvas_hover_label(painter, rect.right_top(), color, &warnings.join("\n"));
        }
    }

    fn draw_object_erase_preview(
        &self,
        canvas_rect: Rect,
        painter: &egui::Painter,
        layer: LayerKind,
        x: i32,
        y: i32,
    ) {
        let rect = self.tile_screen_rect(canvas_rect, x, y, 1, 1);
        let locked = self.layer_state(layer).locked;
        let color = if locked { THEME_ERROR } else { THEME_WARNING };
        self.paint_preview_rect(painter, rect, color, locked);
        if locked {
            self.draw_canvas_hover_label(
                painter,
                rect.right_top(),
                color,
                &format!("{} 已锁定", layer.zh_label()),
            );
        }
    }

    fn paint_preview_rect(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        color: Color32,
        warning: bool,
    ) {
        let alpha = if warning { 46 } else { 32 };
        painter.rect_filled(
            rect,
            0.0,
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha),
        );
        painter.rect_stroke(rect, 0.0, Stroke::new(2.0, color), StrokeKind::Inside);
    }

    fn draw_canvas_hover_label(
        &self,
        painter: &egui::Painter,
        anchor: Pos2,
        color: Color32,
        text: &str,
    ) {
        let pos = anchor + vec2(10.0, 10.0);
        let font = egui::TextStyle::Small.resolve(&egui::Style::default());
        painter.text(
            pos + vec2(1.0, 1.0),
            egui::Align2::LEFT_TOP,
            text,
            font.clone(),
            Color32::from_rgba_unmultiplied(0, 0, 0, 180),
        );
        painter.text(pos, egui::Align2::LEFT_TOP, text, font, color);
    }

    fn asset_placement_warnings(&self, asset: &AssetEntry, layer: LayerKind) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.layer_state(layer).locked {
            warnings.push(format!("{} 已锁定", layer.zh_label()));
        }
        if let Some(expected) = expected_asset_kind_for_layer(layer) {
            if asset.kind != expected {
                warnings.push(format!(
                    "素材类型是{}，当前层需要{}",
                    asset.kind.zh_label(),
                    expected.zh_label()
                ));
            }
        }
        if asset.default_layer != layer {
            warnings.push(format!("素材默认层是{}", asset.default_layer.zh_label()));
        }
        warnings
    }

    fn asset_preview_footprint(&self, asset: &AssetEntry) -> Option<[i32; 2]> {
        asset
            .footprint
            .or_else(|| infer_tile_footprint(asset.default_size, self.document.tile_size))
            .filter(|[width, height]| *width > 1 || *height > 1)
    }

    fn map_screen_rect(&self, canvas_rect: Rect) -> Rect {
        let tile_size = self.document.tile_size as f32;
        Rect::from_min_size(
            self.world_to_screen(canvas_rect, vec2(0.0, 0.0)),
            vec2(
                self.document.width as f32 * tile_size,
                self.document.height as f32 * tile_size,
            ) * self.zoom,
        )
    }

    pub(crate) fn draw_rectangle_preview(&self, canvas_rect: Rect, painter: &egui::Painter) {
        if self.tool != ToolKind::Rectangle {
            return;
        }
        let Some(start) = self.rectangle_drag_start else {
            return;
        };
        let Some(end) = self.rectangle_drag_current.or(self.mouse_tile) else {
            return;
        };

        let min_x = start[0].min(end[0]);
        let max_x = start[0].max(end[0]);
        let min_y = start[1].min(end[1]);
        let max_y = start[1].max(end[1]);
        let rect = self.tile_screen_rect(
            canvas_rect,
            min_x,
            min_y,
            max_x - min_x + 1,
            max_y - min_y + 1,
        );
        painter.rect_filled(
            rect,
            0.0,
            Color32::from_rgba_unmultiplied(
                THEME_WARNING.r(),
                THEME_WARNING.g(),
                THEME_WARNING.b(),
                30,
            ),
        );
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(2.0, THEME_WARNING),
            StrokeKind::Inside,
        );
    }

    pub(crate) fn draw_zone_draft(
        &self,
        canvas_rect: Rect,
        painter: &egui::Painter,
        hover_pos: Option<Pos2>,
        modifiers: Modifiers,
    ) {
        if self.zone_draft_points.is_empty() {
            return;
        }
        let tile_size = self.document.tile_size as f32;
        let mut points = self
            .zone_draft_points
            .iter()
            .map(|point| {
                self.world_to_screen(
                    canvas_rect,
                    vec2(point[0] * tile_size, point[1] * tile_size),
                )
            })
            .collect::<Vec<_>>();
        if let Some(raw) =
            hover_pos.and_then(|hover| self.screen_to_map_position(canvas_rect, hover))
        {
            let point = self.snapped_zone_position(raw, modifiers);
            points.push(self.world_to_screen(
                canvas_rect,
                vec2(point[0] * tile_size, point[1] * tile_size),
            ));
        }

        for pair in points.windows(2) {
            painter.line_segment([pair[0], pair[1]], Stroke::new(2.0, THEME_ACCENT_STRONG));
        }
        for point in points {
            painter.circle_filled(point, 4.5, THEME_ACCENT_STRONG);
        }
    }

    pub(crate) fn draw_status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mouse = self
                .mouse_tile
                .map(|tile| format!("{}, {}", tile[0], tile[1]))
                .unwrap_or_else(|| "-".to_owned());
            let asset = self.selected_asset.as_deref().unwrap_or("none");
            let selections = self.current_selection_list();
            let selected_item = if selections.len() > 1 {
                format!("{} items", selections.len())
            } else {
                selections
                    .first()
                    .map(SelectedItem::label)
                    .unwrap_or_else(|| "none".to_owned())
            };
            let transform = self
                .selected_item
                .as_ref()
                .and_then(|selection| self.transform_for_selection(selection))
                .map(|(flip_x, rotation)| format!("flip_x={}, rot={}deg", flip_x, rotation))
                .unwrap_or_else(|| "-".to_owned());
            let stamp = self
                .stamp_pattern
                .as_ref()
                .map(|pattern| {
                    format!(
                        "{}x{} / {}",
                        pattern.width,
                        pattern.height,
                        pattern.item_count()
                    )
                })
                .unwrap_or_else(|| "-".to_owned());
            let dirty_marker = if self.dirty { "*" } else { "" };
            let current_file = format!(
                "{}{}",
                display_project_path(&self.project_root, &self.map_path),
                dirty_marker
            );

            ui.label(format!("File: {current_file}"));
            ui.separator();
            ui.label(format!("Mouse Tile: {mouse}"));
            ui.separator();
            ui.label(format!("Selected: {asset}"));
            ui.separator();
            ui.label(format!("Selection: {selected_item}"));
            ui.separator();
            ui.label(format!("Transform: {transform}"));
            ui.separator();
            ui.label(format!("Layer: {}", self.active_layer.zh_label()));
            ui.separator();
            ui.label(format!(
                "Ground Size: {}x{}",
                self.ground_footprint_w.max(1),
                self.ground_footprint_h.max(1)
            ));
            ui.separator();
            ui.label(format!("Stamp: {stamp}"));
            ui.separator();
            ui.label(format!("Zoom: {:.0}%", self.zoom * 100.0));
            ui.separator();
            if let Some(texture_status) = self.texture_loading_status() {
                ui.label(texture_status);
                ui.separator();
            }
            ui.label(&self.status);
        });
    }

    pub(crate) fn world_to_screen(&self, canvas_rect: Rect, world: Vec2) -> Pos2 {
        canvas_rect.min + self.pan + world * self.zoom
    }

    pub(crate) fn visible_tile_bounds(&self, canvas_rect: Rect) -> VisibleTileBounds {
        visible_tile_bounds_for_canvas(
            canvas_rect,
            self.pan,
            self.zoom,
            self.document.tile_size as f32,
            self.document.width,
            self.document.height,
        )
    }

    pub(crate) fn screen_to_tile(&self, canvas_rect: Rect, screen: Pos2) -> Option<[i32; 2]> {
        let local = (screen - canvas_rect.min - self.pan) / self.zoom;
        let tile_size = self.document.tile_size as f32;
        let x = (local.x / tile_size).floor() as i32;
        let y = (local.y / tile_size).floor() as i32;

        if x < 0 || y < 0 || x >= self.document.width as i32 || y >= self.document.height as i32 {
            None
        } else {
            Some([x, y])
        }
    }

    pub(crate) fn screen_to_map_position(
        &self,
        canvas_rect: Rect,
        screen: Pos2,
    ) -> Option<[f32; 2]> {
        let local = (screen - canvas_rect.min - self.pan) / self.zoom;
        let tile_size = self.document.tile_size as f32;
        let x = local.x / tile_size;
        let y = local.y / tile_size;

        if x < 0.0 || y < 0.0 || x >= self.document.width as f32 || y >= self.document.height as f32
        {
            None
        } else {
            Some([x, y])
        }
    }

    pub(crate) fn snapped_map_position(
        &self,
        raw: [f32; 2],
        asset: Option<&AssetEntry>,
        modifiers: Modifiers,
    ) -> [f32; 2] {
        let snap = if modifiers.alt {
            SnapMode::Free
        } else if modifiers.shift {
            SnapMode::HalfGrid
        } else {
            asset.map(|asset| asset.snap).unwrap_or(SnapMode::Grid)
        };
        match snap {
            SnapMode::Grid => [raw[0].floor(), raw[1].floor()],
            SnapMode::HalfGrid => [(raw[0] * 2.0).round() * 0.5, (raw[1] * 2.0).round() * 0.5],
            SnapMode::Free => [raw[0], raw[1]],
        }
    }

    pub(crate) fn snapped_zone_position(&self, raw: [f32; 2], modifiers: Modifiers) -> [f32; 2] {
        if modifiers.alt {
            return [
                raw[0].clamp(0.0, self.document.width as f32),
                raw[1].clamp(0.0, self.document.height as f32),
            ];
        }

        let step = if modifiers.shift { 0.5 } else { 0.25 };
        [
            ((raw[0] / step).round() * step).clamp(0.0, self.document.width as f32),
            ((raw[1] / step).round() * step).clamp(0.0, self.document.height as f32),
        ]
    }

    pub(crate) fn snapped_collision_position(
        &self,
        raw: [f32; 2],
        modifiers: Modifiers,
    ) -> [f32; 2] {
        if modifiers.alt {
            return [
                raw[0].clamp(0.0, self.document.width as f32),
                raw[1].clamp(0.0, self.document.height as f32),
            ];
        }

        let step = if modifiers.shift { 0.5 } else { 0.25 };
        [
            ((raw[0] / step).round() * step).clamp(0.0, self.document.width as f32),
            ((raw[1] / step).round() * step).clamp(0.0, self.document.height as f32),
        ]
    }
}

fn expected_asset_kind_for_layer(layer: LayerKind) -> Option<AssetKind> {
    match layer {
        LayerKind::Ground => Some(AssetKind::Tile),
        LayerKind::Decals => Some(AssetKind::Decal),
        LayerKind::Objects => Some(AssetKind::Object),
        LayerKind::Entities => Some(AssetKind::Entity),
        LayerKind::Zones | LayerKind::Collision => None,
    }
}

#[derive(Clone, Copy, Debug)]
enum DirectionSide {
    North,
    East,
    South,
    West,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct VisibleTileBounds {
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
}

fn visible_tile_bounds_for_canvas(
    canvas_rect: Rect,
    pan: Vec2,
    zoom: f32,
    tile_size: f32,
    map_width: u32,
    map_height: u32,
) -> VisibleTileBounds {
    if map_width == 0 || map_height == 0 || tile_size <= f32::EPSILON {
        return VisibleTileBounds {
            min_x: 0,
            min_y: 0,
            max_x: 0,
            max_y: 0,
        };
    }

    let zoom = zoom.max(0.01);
    let local_min = (canvas_rect.min - canvas_rect.min - pan) / zoom;
    let local_max = (canvas_rect.max - canvas_rect.min - pan) / zoom;
    let width = map_width as i32;
    let height = map_height as i32;

    VisibleTileBounds {
        min_x: ((local_min.x / tile_size).floor() as i32 - 1).clamp(0, width),
        min_y: ((local_min.y / tile_size).floor() as i32 - 1).clamp(0, height),
        max_x: ((local_max.x / tile_size).ceil() as i32 + 1).clamp(0, width),
        max_y: ((local_max.y / tile_size).ceil() as i32 + 1).clamp(0, height),
    }
}

fn tile_intersects_rect(
    tile: &content::TileInstance,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
) -> bool {
    let tile_max_x = tile.x + tile.w.max(1);
    let tile_max_y = tile.y + tile.h.max(1);
    tile.x < max_x && tile_max_x > min_x && tile.y < max_y && tile_max_y > min_y
}

fn bounds_intersects_tile_rect(x: f32, y: f32, w: f32, h: f32, visible: VisibleTileBounds) -> bool {
    x < visible.max_x as f32
        && x + w.max(0.0) > visible.min_x as f32
        && y < visible.max_y as f32
        && y + h.max(0.0) > visible.min_y as f32
}

fn tile_contains_cell(tile: &content::TileInstance, x: i32, y: i32) -> bool {
    let width = tile.w.max(1);
    let height = tile.h.max(1);
    x >= tile.x && x < tile.x + width && y >= tile.y && y < tile.y + height
}

fn normalized_tile_rect(start: [i32; 2], end: [i32; 2]) -> [i32; 4] {
    [
        start[0].min(end[0]),
        start[1].min(end[1]),
        start[0].max(end[0]),
        start[1].max(end[1]),
    ]
}

fn instance_anchor_in_rect(x: f32, y: f32, min_x: i32, min_y: i32, max_x: i32, max_y: i32) -> bool {
    let tile_x = x.floor() as i32;
    let tile_y = y.floor() as i32;
    tile_x >= min_x && tile_x <= max_x && tile_y >= min_y && tile_y <= max_y
}

fn stamp_item_bounds(item: &StampItem) -> Option<[i32; 4]> {
    Some(match item {
        StampItem::Ground(tile) => [
            tile.x,
            tile.y,
            tile.x + tile.w.max(1),
            tile.y + tile.h.max(1),
        ],
        StampItem::Decal(instance) | StampItem::Object(instance) => {
            let x = instance.x.floor() as i32;
            let y = instance.y.floor() as i32;
            [x, y, x + 1, y + 1]
        }
        StampItem::Entity(instance) => {
            let x = instance.x.floor() as i32;
            let y = instance.y.floor() as i32;
            [x, y, x + 1, y + 1]
        }
    })
}

fn offset_stamp_item(item: &mut StampItem, dx: i32, dy: i32) {
    match item {
        StampItem::Ground(tile) => {
            tile.x += dx;
            tile.y += dy;
        }
        StampItem::Decal(instance) | StampItem::Object(instance) => {
            instance.x += dx as f32;
            instance.y += dy as f32;
        }
        StampItem::Entity(instance) => {
            instance.x += dx as f32;
            instance.y += dy as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_tile_bounds_tracks_pan_and_zoom() {
        let canvas = Rect::from_min_size(Pos2::new(0.0, 0.0), vec2(320.0, 320.0));

        assert_eq!(
            visible_tile_bounds_for_canvas(canvas, vec2(0.0, 0.0), 1.0, 32.0, 100, 100),
            VisibleTileBounds {
                min_x: 0,
                min_y: 0,
                max_x: 11,
                max_y: 11,
            }
        );
        assert_eq!(
            visible_tile_bounds_for_canvas(canvas, vec2(-320.0, -160.0), 1.0, 32.0, 100, 100),
            VisibleTileBounds {
                min_x: 9,
                min_y: 4,
                max_x: 21,
                max_y: 16,
            }
        );
    }

    #[test]
    fn bounds_intersection_uses_exclusive_visible_max() {
        let visible = VisibleTileBounds {
            min_x: 10,
            min_y: 5,
            max_x: 20,
            max_y: 15,
        };

        assert!(bounds_intersects_tile_rect(19.5, 14.5, 1.0, 1.0, visible));
        assert!(!bounds_intersects_tile_rect(20.0, 14.5, 1.0, 1.0, visible));
        assert!(!bounds_intersects_tile_rect(9.0, 14.5, 1.0, 1.0, visible));
    }

    #[test]
    fn tile_intersection_needs_expanded_max_to_include_right_and_bottom_neighbors() {
        let right_neighbor = tile_at(11, 10);
        let bottom_neighbor = tile_at(10, 11);

        assert!(!tile_intersects_rect(&right_neighbor, 9, 9, 11, 11));
        assert!(!tile_intersects_rect(&bottom_neighbor, 9, 9, 11, 11));
        assert!(tile_intersects_rect(&right_neighbor, 9, 9, 12, 12));
        assert!(tile_intersects_rect(&bottom_neighbor, 9, 9, 12, 12));
    }

    #[test]
    fn align_delta_uses_requested_group_edge_or_center() {
        let item = SelectionMapBounds::from_min_max(2.0, 4.0, 6.0, 10.0);

        assert_eq!(align_delta(BatchAlignMode::Left, item, 1.0), [-1.0, 0.0]);
        assert_eq!(align_delta(BatchAlignMode::Right, item, 12.0), [6.0, 0.0]);
        assert_eq!(align_delta(BatchAlignMode::CenterX, item, 8.0), [4.0, 0.0]);
        assert_eq!(align_delta(BatchAlignMode::Top, item, 3.0), [0.0, -1.0]);
        assert_eq!(align_delta(BatchAlignMode::Bottom, item, 12.0), [0.0, 2.0]);
        assert_eq!(align_delta(BatchAlignMode::CenterY, item, 9.0), [0.0, 2.0]);
    }

    #[test]
    fn zone_bounds_cover_all_points() {
        let bounds = zone_map_bounds(&[[3.0, 4.0], [1.0, 8.0], [5.0, 2.0]]).unwrap();

        assert_eq!(bounds.min_x, 1.0);
        assert_eq!(bounds.min_y, 2.0);
        assert_eq!(bounds.max_x, 5.0);
        assert_eq!(bounds.max_y, 8.0);
    }

    #[test]
    fn distribute_deltas_preserve_outer_span_with_equal_gaps() {
        let moves = distribute_deltas(
            BatchDistributeMode::Horizontal,
            vec![
                (0, SelectionMapBounds::from_min_max(0.0, 0.0, 1.0, 1.0)),
                (1, SelectionMapBounds::from_min_max(4.0, 0.0, 5.0, 1.0)),
                (2, SelectionMapBounds::from_min_max(10.0, 0.0, 12.0, 1.0)),
            ],
        );

        assert_eq!(moves, vec![(1, [1.0, 0.0])]);
    }

    #[test]
    fn distribute_deltas_vertical_uses_current_order() {
        let moves = distribute_deltas(
            BatchDistributeMode::Vertical,
            vec![
                (0, SelectionMapBounds::from_min_max(0.0, 0.0, 1.0, 2.0)),
                (1, SelectionMapBounds::from_min_max(0.0, 6.0, 1.0, 7.0)),
                (2, SelectionMapBounds::from_min_max(0.0, 9.0, 1.0, 10.0)),
            ],
        );

        assert_eq!(moves, vec![(1, [0.0, -1.0])]);
    }

    #[test]
    fn common_optional_text_value_reports_mixed() {
        assert_eq!(
            common_optional_text_value(vec![Some("a".to_owned()), Some("a".to_owned())]),
            ("a".to_owned(), false)
        );
        assert_eq!(
            common_optional_text_value(vec![Some("a".to_owned()), None]),
            (String::new(), true)
        );
        assert_eq!(
            common_optional_text_value(vec![None, None]),
            (String::new(), false)
        );
    }

    #[test]
    fn trimmed_optional_drops_blank_values() {
        assert_eq!(
            trimmed_optional("  codex.entry  "),
            Some("codex.entry".to_owned())
        );
        assert_eq!(trimmed_optional("   "), None);
    }

    fn tile_at(x: i32, y: i32) -> content::TileInstance {
        content::TileInstance {
            asset: "ow_tile_test".to_owned(),
            x,
            y,
            w: 1,
            h: 1,
            flip_x: false,
            rotation: 0,
        }
    }
}
