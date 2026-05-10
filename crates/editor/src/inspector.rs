use super::*;

impl EditorApp {
    pub(super) fn draw_inspector_panel(&mut self, ui: &mut egui::Ui) {
        if panel_header(ui, "Inspector", ">", "收起右侧栏") {
            self.show_right_sidebar = false;
        }

        let selections = self.current_selection_list();
        if selections.len() > 1 {
            self.draw_multi_selection_inspector(ui, selections);
        } else if let Some(selection) = selections.into_iter().next() {
            self.draw_selection_inspector(ui, selection);
        } else if let Some(asset) = self.selected_asset().cloned() {
            self.draw_asset_inspector(ui, &asset);
        } else {
            self.draw_map_inspector(ui);
        }
    }

    fn draw_multi_selection_inspector(&mut self, ui: &mut egui::Ui, selections: Vec<SelectedItem>) {
        ui.label(format!("多选：{} 个对象", selections.len()));
        for layer in LayerKind::ALL {
            let count = selections
                .iter()
                .filter(|selection| selection.layer == layer)
                .count();
            if count > 0 {
                ui.label(format!("{}：{}", layer.zh_label(), count));
            }
        }

        if selections
            .iter()
            .any(|selection| self.layer_state(selection.layer).locked)
        {
            ui.colored_label(THEME_WARNING, "部分所选图层已锁定，批量编辑会跳过它们");
        }

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("复制").clicked() {
                self.copy_selected_item();
            }
            if ui.button("复制一份").clicked() {
                self.duplicate_selected_item();
            }
            if ui.button("删除").clicked() {
                self.delete_current_selection();
            }
        });

        let alignable_count = self.alignable_selection_count(&selections);
        if alignable_count > 1 {
            ui.separator();
            inspector_section(ui, "批量对齐");
            ui.label(format!("{} 个可对齐对象", alignable_count));
            ui.horizontal_wrapped(|ui| {
                for mode in BatchAlignMode::ALL {
                    if ui.button(mode.label()).clicked() {
                        self.align_selected_items(mode);
                    }
                }
            });
            ui.small("方向键微调；Shift = 半格，Ctrl = 4 格。");
        }
        let distributable_count = self.distributable_selection_count(&selections);
        if distributable_count > 2 {
            ui.horizontal_wrapped(|ui| {
                for mode in BatchDistributeMode::ALL {
                    if ui.button(mode.label()).clicked() {
                        self.distribute_selected_items(mode);
                    }
                }
            });
        }

        if let Some(asset) = self.selected_asset().cloned() {
            let replaceable_count = self.replaceable_selection_count(&selections, &asset);
            if replaceable_count > 0 {
                ui.separator();
                inspector_section(ui, "批量替换");
                ui.label(format!("当前素材：{}", asset.id));
                if ui
                    .button(format!("替换 {} 个匹配对象", replaceable_count))
                    .clicked()
                {
                    self.replace_selected_assets_with_current();
                }
            }
        }

        let entity_count = self.editable_entity_selection_count(&selections);
        if entity_count > 0 {
            ui.separator();
            inspector_section(ui, "批量实体字段");
            ui.label(format!("{} 个实体可编辑", entity_count));
            let (mut entity_type, mixed) = self.common_entity_type_for_selection(&selections);
            if mixed {
                ui.colored_label(THEME_MUTED_TEXT, "当前实体类型：混合");
            }
            if labeled_text_edit_with_options(
                ui,
                "实体类型",
                "batch_entity_type",
                &mut entity_type,
                &self.entity_type_options(),
            ) {
                self.set_entity_type_for_selection(&selections, entity_type);
            }
        }

        let unlockable_count = self.unlockable_selection_count(&selections);
        if unlockable_count > 0 {
            ui.separator();
            inspector_section(ui, "批量解锁条件");
            ui.label(format!("{} 个实体/区域可编辑", unlockable_count));

            let codex_id_options = self.codex_id_options();
            let item_id_options = self.item_id_options();

            let (mut codex_id, codex_mixed) = self.common_unlock_codex_for_selection(&selections);
            if codex_mixed {
                ui.colored_label(THEME_MUTED_TEXT, "扫描需求：混合");
            }
            if labeled_text_edit_with_options(
                ui,
                "扫描需求",
                "batch_unlock_codex",
                &mut codex_id,
                &codex_id_options,
            ) {
                self.set_unlock_codex_for_selection(&selections, codex_id.clone());
            }
            if !codex_mixed && !codex_id.trim().is_empty() {
                draw_codex_entry_preview(ui, codex_id.trim(), self.codex_database.as_ref());
            }

            let (mut item_id, item_mixed) = self.common_unlock_item_for_selection(&selections);
            if item_mixed {
                ui.colored_label(THEME_MUTED_TEXT, "物品需求：混合");
            }
            if labeled_text_edit_with_options(
                ui,
                "物品需求",
                "batch_unlock_item",
                &mut item_id,
                &item_id_options,
            ) {
                self.set_unlock_item_for_selection(&selections, item_id);
            }

            let (mut locked_message, message_mixed) =
                self.common_unlock_message_for_selection(&selections);
            if message_mixed {
                ui.colored_label(THEME_MUTED_TEXT, "锁定提示：混合");
            }
            if labeled_text_edit(ui, "锁定提示", &mut locked_message) {
                self.set_unlock_message_for_selection(&selections, locked_message);
            }

            ui.horizontal(|ui| {
                if ui.button("清除解锁条件").clicked() {
                    self.clear_unlock_for_selection(&selections);
                }
            });
            ui.small("空值会清除对应字段；混合值时输入会覆盖所有可编辑对象。");
        }

        let object_like_count = selections
            .iter()
            .filter(|selection| {
                matches!(
                    selection.layer,
                    LayerKind::Decals | LayerKind::Objects | LayerKind::Entities
                ) && !self.layer_state(selection.layer).locked
            })
            .count();
        if object_like_count > 0 {
            ui.separator();
            ui.label(format!("批量层级：{} 个可调对象", object_like_count));
            let mut z_index = 0;
            if ui
                .add(egui::DragValue::new(&mut z_index).prefix("层级 "))
                .changed()
            {
                self.push_undo_snapshot();
                for selection in &selections {
                    self.set_z_index_for_selection(selection, z_index);
                }
                self.mark_dirty();
            }
        }
    }

    fn draw_map_inspector(&mut self, ui: &mut egui::Ui) {
        inspector_section(ui, "地图属性");
        let mut next = self.document.clone();
        let mut changed = false;
        changed |= labeled_text_edit(ui, "地图 ID", &mut next.id);
        changed |= labeled_text_edit(ui, "模式", &mut next.mode);
        changed |= ui
            .add(
                egui::DragValue::new(&mut next.width)
                    .range(1..=512)
                    .prefix("宽 "),
            )
            .changed();
        changed |= ui
            .add(
                egui::DragValue::new(&mut next.height)
                    .range(1..=512)
                    .prefix("高 "),
            )
            .changed();
        changed |= ui
            .add(
                egui::DragValue::new(&mut next.tile_size)
                    .range(1..=256)
                    .prefix("格 "),
            )
            .changed();

        inspector_section(ui, "Codex 数据");
        ui.small(display_project_path(
            &self.project_root,
            &self.codex_db_path(),
        ));
        match &self.codex_database {
            Some(database) => {
                ui.label(format!("{} 个图鉴条目", database.entries().len()));
            }
            None => {
                ui.colored_label(THEME_WARNING, &self.codex_db_status);
            }
        }
        if ui.button("重新加载 Codex").clicked() {
            self.reload_codex_database();
        }

        inspector_section(ui, "出生点");
        for spawn in &mut next.spawns {
            changed |= labeled_text_edit(ui, "出生点 ID", &mut spawn.id);
            ui.horizontal(|ui| {
                changed |= ui
                    .add(egui::DragValue::new(&mut spawn.x).speed(0.1).prefix("x "))
                    .changed();
                changed |= ui
                    .add(egui::DragValue::new(&mut spawn.y).speed(0.1).prefix("y "))
                    .changed();
            });
        }

        if changed {
            self.push_undo_snapshot();
            self.document = next;
            self.save_as_id = self.document.id.clone();
            self.mark_dirty();
        }

        self.draw_transition_links_inspector(ui);
    }

    fn draw_asset_inspector(&mut self, ui: &mut egui::Ui, asset: &AssetEntry) {
        ui.label("素材默认属性");
        ui.monospace(&asset.id);
        ui.label(format!("类型：{}", asset.kind.zh_label()));
        ui.label(format!("分类：{}", category_label(&asset.category)));
        ui.label(format!("默认图层：{}", asset.default_layer.zh_label()));
        ui.label(format!("锚点：{}", anchor_label(asset.anchor)));
        ui.label(format!("吸附：{}", snap_label(asset.snap)));
        ui.label(format!(
            "默认尺寸：{:.1} x {:.1}",
            asset.default_size[0], asset.default_size[1]
        ));
        if asset.kind == AssetKind::Tile {
            let footprint = self.asset_tile_footprint(asset);
            ui.label(format!("占格：{} x {}", footprint[0], footprint[1]));
        }
        if let Some(rect) = asset.default_collision_rect {
            ui.label(format!(
                "默认碰撞：{:.2},{:.2} / {:.2}x{:.2}",
                rect.offset[0], rect.offset[1], rect.size[0], rect.size[1]
            ));
        }
        if let Some(rect) = asset.default_interaction_rect {
            ui.label(format!(
                "默认交互：{:.2},{:.2} / {:.2}x{:.2}",
                rect.offset[0], rect.offset[1], rect.size[0], rect.size[1]
            ));
        }
        if let Some(entity_type) = &asset.entity_type {
            ui.label(format!("实体类型：{entity_type}"));
        }
        if let Some(codex_id) = &asset.codex_id {
            ui.label(format!("图鉴：{codex_id}"));
        }
        if !asset.tags.is_empty() {
            ui.label(format!("Tags：{}", asset.tags.join(", ")));
        }
        ui.small(&asset.relative_path);
        draw_asset_scan_status(ui, asset, self.codex_database.as_ref());
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("编辑素材").clicked() {
                self.open_edit_asset_dialog(&asset.id);
            }
            if ui.button("从素材库移除").clicked() {
                let ctx = ui.ctx().clone();
                self.delete_selected_asset_definition(&ctx);
            }
        });
        if self.asset_db_dirty {
            ui.colored_label(THEME_WARNING, "素材库有未保存修改");
            if ui.button("保存素材库").clicked() {
                self.save_asset_database();
            }
        }
    }

    fn draw_selection_inspector(&mut self, ui: &mut egui::Ui, selection: SelectedItem) {
        ui.label(format!("选中：{}", selection.label()));
        if self.layer_state(selection.layer).locked {
            ui.colored_label(THEME_WARNING, "当前图层已锁定");
        }

        let entity_type_options = self.entity_type_options();
        let codex_id_options = self.codex_id_options();
        let item_id_options = self.item_id_options();
        let map_path_options = self.map_path_options();
        let mut next = self.document.clone();
        let mut changed = false;
        let mut next_selection = selection.clone();
        let mut open_transition_target = None;

        match selection.layer {
            LayerKind::Ground => {
                if let Some([x, y]) = parse_ground_selection_id(&selection.id) {
                    if let Some(tile) = next
                        .layers
                        .ground
                        .iter_mut()
                        .find(|tile| tile.x == x && tile.y == y)
                    {
                        changed |= labeled_text_edit(ui, "素材 ID", &mut tile.asset);
                        changed |= ui
                            .add(egui::DragValue::new(&mut tile.x).prefix("x "))
                            .changed();
                        changed |= ui
                            .add(egui::DragValue::new(&mut tile.y).prefix("y "))
                            .changed();
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut tile.w)
                                    .range(1..=512)
                                    .prefix("w "),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut tile.h)
                                    .range(1..=512)
                                    .prefix("h "),
                            )
                            .changed();
                        changed |= ui.checkbox(&mut tile.flip_x, "水平翻转").changed();
                        changed |= ui
                            .add(egui::DragValue::new(&mut tile.rotation).prefix("旋转 "))
                            .changed();
                        next_selection.id = ground_selection_id(tile.x, tile.y);
                    }
                }
            }
            LayerKind::Decals => {
                if let Some(instance) = next
                    .layers
                    .decals
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    let asset = self.registry.get(&instance.asset).cloned();
                    let default_size = asset.as_ref().map(|asset| asset.default_size);
                    changed |= object_instance_editor(
                        ui,
                        instance,
                        default_size,
                        &mut self.lock_aspect_ratio,
                        false,
                    );
                    draw_object_layer_scan_status(
                        ui,
                        selection.layer,
                        instance,
                        asset.as_ref(),
                        self.codex_database.as_ref(),
                    );
                    next_selection.id = instance.id.clone();
                }
            }
            LayerKind::Objects => {
                if let Some(instance) = next
                    .layers
                    .objects
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    let asset = self.registry.get(&instance.asset).cloned();
                    let default_size = asset.as_ref().map(|asset| asset.default_size);
                    changed |= object_instance_editor(
                        ui,
                        instance,
                        default_size,
                        &mut self.lock_aspect_ratio,
                        true,
                    );
                    draw_object_layer_scan_status(
                        ui,
                        selection.layer,
                        instance,
                        asset.as_ref(),
                        self.codex_database.as_ref(),
                    );
                    next_selection.id = instance.id.clone();
                }
            }
            LayerKind::Entities => {
                if let Some(instance) = next
                    .layers
                    .entities
                    .iter_mut()
                    .find(|instance| instance.id == selection.id)
                {
                    let asset = self.registry.get(&instance.asset).cloned();
                    changed |= labeled_text_edit(ui, "实例 ID", &mut instance.id);
                    changed |= labeled_text_edit(ui, "素材 ID", &mut instance.asset);
                    changed |= labeled_text_edit_with_options(
                        ui,
                        "实体类型",
                        format!("entity_type_{}", instance.id),
                        &mut instance.entity_type,
                        &entity_type_options,
                    );
                    draw_entity_scan_status(
                        ui,
                        instance,
                        asset.as_ref(),
                        self.codex_database.as_ref(),
                    );
                    let unlock_id = format!("entity_unlock_{}", instance.id);
                    draw_unlock_rule_editor(
                        ui,
                        "解锁条件",
                        &unlock_id,
                        &mut instance.unlock,
                        &codex_id_options,
                        &item_id_options,
                        self.codex_database.as_ref(),
                        &mut changed,
                    );
                    let transition_id = format!("entity_transition_{}", instance.id);
                    draw_transition_target_editor(
                        ui,
                        "转场目标",
                        &transition_id,
                        &mut instance.transition,
                        &map_path_options,
                        &mut changed,
                    );
                    if self.draw_transition_target_action(ui, instance.transition.as_ref()) {
                        open_transition_target = instance.transition.clone();
                    }
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut instance.x)
                                .speed(0.1)
                                .prefix("x "),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut instance.y)
                                .speed(0.1)
                                .prefix("y "),
                        )
                        .changed();
                    let default_size = asset.as_ref().map(|asset| asset.default_size);
                    changed |= instance_size_editor(
                        ui,
                        &mut instance.scale_x,
                        &mut instance.scale_y,
                        default_size,
                        &mut self.lock_aspect_ratio,
                    );
                    changed |= ui
                        .add(egui::DragValue::new(&mut instance.z_index).prefix("层级 "))
                        .changed();
                    entity_rect_editor(ui, "遮挡/排序范围", &mut instance.depth_rect, &mut changed);
                    entity_rect_editor(ui, "碰撞范围", &mut instance.collision_rect, &mut changed);
                    entity_rect_editor(
                        ui,
                        "交互范围",
                        &mut instance.interaction_rect,
                        &mut changed,
                    );
                    changed |= ui.checkbox(&mut instance.flip_x, "水平翻转").changed();
                    changed |= ui
                        .add(egui::DragValue::new(&mut instance.rotation).prefix("旋转 "))
                        .changed();
                    next_selection.id = instance.id.clone();
                }
            }
            LayerKind::Zones => {
                if let Some(zone) = next
                    .layers
                    .zones
                    .iter_mut()
                    .find(|zone| zone.id == selection.id)
                {
                    changed |= labeled_text_edit(ui, "区域 ID", &mut zone.id);
                    changed |= labeled_text_edit(ui, "区域类型", &mut zone.zone_type);
                    changed |= draw_zone_type_preset_checkboxes(ui, zone);

                    if zone.zone_type == "WalkSurface" || zone.surface.is_some() {
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label("可走表面");
                            if zone.surface.is_none() && ui.button("添加").clicked() {
                                zone.surface = Some(content::WalkSurfaceRule::default());
                                if zone.zone_type == "Trigger" {
                                    zone.zone_type = "WalkSurface".to_owned();
                                }
                                changed = true;
                            }
                            if zone.surface.is_some() && ui.button("清除").clicked() {
                                zone.surface = None;
                                changed = true;
                            }
                        });
                        if let Some(surface) = &mut zone.surface {
                            let mut surface_id = surface.surface_id.clone().unwrap_or_default();
                            if labeled_text_edit(ui, "Surface ID", &mut surface_id) {
                                set_optional_string(&mut surface.surface_id, surface_id);
                                changed = true;
                            }
                            egui::ComboBox::from_label("表面类型")
                                .selected_text(surface.kind.zh_label())
                                .show_ui(ui, |ui| {
                                    changed |= ui
                                        .selectable_value(
                                            &mut surface.kind,
                                            content::WalkSurfaceKind::Platform,
                                            "台面 / 平台",
                                        )
                                        .changed();
                                    changed |= ui
                                        .selectable_value(
                                            &mut surface.kind,
                                            content::WalkSurfaceKind::Ramp,
                                            "斜坡入口 / 出口",
                                        )
                                        .changed();
                                });
                            changed |= ui
                                .checkbox(&mut surface.constrain_movement, "限制在同一表面区域内")
                                .changed();
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut surface.z_index)
                                        .range(-256..=256)
                                        .prefix("表面层级 "),
                                )
                                .changed();
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut surface.depth_offset)
                                        .range(-512.0..=512.0)
                                        .speed(1.0)
                                        .prefix("深度偏移 "),
                                )
                                .changed();
                            ui.colored_label(
                                THEME_MUTED_TEXT,
                                "圆台顶面和斜坡填同一个 Surface ID；只有斜坡入口能从地面切入台面。",
                            );
                        }
                    }

                    if zone.zone_type == "HazardZone" || zone.hazard.is_some() {
                        ui.separator();
                        draw_zone_hazard_editor(ui, zone, &mut changed);
                    }

                    if zone.zone_type == "PromptZone" || zone.prompt.is_some() {
                        ui.separator();
                        draw_zone_prompt_editor(ui, zone, &mut changed);
                    }

                    if matches!(zone.zone_type.as_str(), "ObjectiveZone" | "Checkpoint")
                        || zone.objective.is_some()
                    {
                        ui.separator();
                        draw_zone_objective_editor(ui, zone, &mut changed);
                    }

                    let unlock_id = format!("zone_unlock_{}", zone.id);
                    draw_unlock_rule_editor(
                        ui,
                        "解锁条件",
                        &unlock_id,
                        &mut zone.unlock,
                        &codex_id_options,
                        &item_id_options,
                        self.codex_database.as_ref(),
                        &mut changed,
                    );
                    let transition_id = format!("zone_transition_{}", zone.id);
                    draw_transition_target_editor(
                        ui,
                        "转场目标",
                        &transition_id,
                        &mut zone.transition,
                        &map_path_options,
                        &mut changed,
                    );
                    if self.draw_transition_target_action(ui, zone.transition.as_ref()) {
                        open_transition_target = zone.transition.clone();
                    }
                    next_selection.id = zone.id.clone();
                    ui.label(format!("点数：{}", zone.points.len()));
                    for (index, point) in zone.points.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("#{index}"));
                            changed |= ui
                                .add(egui::DragValue::new(&mut point[0]).speed(0.05).prefix("x "))
                                .changed();
                            changed |= ui
                                .add(egui::DragValue::new(&mut point[1]).speed(0.05).prefix("y "))
                                .changed();
                        });
                    }
                    ui.horizontal(|ui| {
                        if ui.button("加点").clicked() {
                            let point = zone.points.last().copied().unwrap_or([0.0, 0.0]);
                            zone.points.push([point[0] + 1.0, point[1]]);
                            changed = true;
                        }
                        if ui.button("删末点").clicked() && zone.points.len() > 3 {
                            zone.points.pop();
                            changed = true;
                        }
                        if ui.button("反向").clicked() {
                            zone.points.reverse();
                            changed = true;
                        }
                    });
                }
            }
            LayerKind::Collision => {
                ui.label("碰撞格请用碰撞工具绘制或擦除");
            }
        }

        if changed && !self.layer_state(selection.layer).locked {
            self.push_undo_snapshot();
            self.document = next;
            self.set_single_selection(Some(next_selection));
            self.mark_dirty();
        }

        if let Some(transition) = open_transition_target {
            self.open_transition_target_map(&transition);
        }
    }

    fn draw_transition_target_action(
        &self,
        ui: &mut egui::Ui,
        transition: Option<&content::TransitionTarget>,
    ) -> bool {
        let Some(transition) = transition else {
            return false;
        };
        match self.transition_target_map_path(transition) {
            Ok(path) => {
                ui.horizontal(|ui| {
                    let clicked = ui.button("打开目标地图").clicked();
                    ui.small(display_project_path(&self.project_root, &path));
                    clicked
                })
                .inner
            }
            Err(error) => {
                ui.colored_label(THEME_WARNING, format!("目标地图不可打开：{error}"));
                false
            }
        }
    }

    fn draw_transition_links_inspector(&mut self, ui: &mut egui::Ui) {
        let links = self.transition_link_entries();
        ui.separator();
        inspector_section(ui, "转场关系");
        if links.is_empty() {
            ui.colored_label(THEME_MUTED_TEXT, "当前地图没有实体/区域转场目标。");
            return;
        }

        for link in links {
            ui.separator();
            ui.label(&link.source);
            ui.small(format!(
                "scene: {} | map: {} | spawn: {}",
                link.scene, link.map_path, link.spawn_id
            ));
            if let Some(problem) = &link.problem {
                ui.colored_label(THEME_WARNING, problem);
            } else if link.target_path.is_some() {
                ui.colored_label(THEME_ACCENT_STRONG, "目标地图可打开");
            } else {
                ui.colored_label(THEME_MUTED_TEXT, "未指定目标地图，运行时会使用默认路径。");
            }
            if let Some(path) = link.target_path {
                if ui.button("打开目标地图").clicked() {
                    let focus_spawn = (link.spawn_id != "-").then_some(link.spawn_id);
                    self.request_open_map(path, focus_spawn);
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ZoneTypePreset {
    WalkSurface,
    CollisionArea,
    CollisionLine,
    MapTransition,
    HazardZone,
    PromptZone,
    ObjectiveZone,
    Checkpoint,
}

impl ZoneTypePreset {
    const ALL: [Self; 8] = [
        Self::WalkSurface,
        Self::CollisionArea,
        Self::CollisionLine,
        Self::MapTransition,
        Self::HazardZone,
        Self::PromptZone,
        Self::ObjectiveZone,
        Self::Checkpoint,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::WalkSurface => "可走表面",
            Self::CollisionArea => "碰撞区域",
            Self::CollisionLine => "碰撞线",
            Self::MapTransition => "转场",
            Self::HazardZone => "危险区",
            Self::PromptZone => "提示区",
            Self::ObjectiveZone => "目标区",
            Self::Checkpoint => "检查点",
        }
    }

    fn zone_type(self) -> &'static str {
        match self {
            Self::WalkSurface => "WalkSurface",
            Self::CollisionArea => "CollisionArea",
            Self::CollisionLine => "CollisionLine",
            Self::MapTransition => "MapTransition",
            Self::HazardZone => "HazardZone",
            Self::PromptZone => "PromptZone",
            Self::ObjectiveZone => "ObjectiveZone",
            Self::Checkpoint => "Checkpoint",
        }
    }
}

fn draw_zone_type_preset_checkboxes(ui: &mut egui::Ui, zone: &mut content::ZoneInstance) -> bool {
    let mut changed = false;
    ui.label("快速类型");
    ui.horizontal_wrapped(|ui| {
        for preset in ZoneTypePreset::ALL {
            let mut selected = zone.zone_type == preset.zone_type();
            if ui.checkbox(&mut selected, preset.label()).clicked() {
                changed |= apply_zone_type_preset(zone, preset);
            }
        }
    });
    changed
}

fn apply_zone_type_preset(zone: &mut content::ZoneInstance, preset: ZoneTypePreset) -> bool {
    let mut changed = zone.zone_type != preset.zone_type();
    zone.zone_type = preset.zone_type().to_owned();

    match preset {
        ZoneTypePreset::WalkSurface => {
            if zone.surface.is_none() {
                zone.surface = Some(content::WalkSurfaceRule::default());
                changed = true;
            }
        }
        ZoneTypePreset::CollisionArea
        | ZoneTypePreset::CollisionLine
        | ZoneTypePreset::MapTransition => {
            if zone.surface.take().is_some() {
                changed = true;
            }
        }
        ZoneTypePreset::HazardZone => {
            if zone.hazard.is_none() {
                zone.hazard = Some(content::HazardRule {
                    effects: vec![content::HazardEffect::new("oxygen", -2.0)],
                    message: None,
                });
                changed = true;
            }
        }
        ZoneTypePreset::PromptZone => {
            if zone.prompt.is_none() {
                zone.prompt = Some(content::PromptRule::default());
                changed = true;
            }
        }
        ZoneTypePreset::ObjectiveZone | ZoneTypePreset::Checkpoint => {
            if zone.objective.is_none() {
                zone.objective = Some(content::ObjectiveRule::default());
                changed = true;
            }
        }
    }

    changed
}

fn draw_zone_hazard_editor(
    ui: &mut egui::Ui,
    zone: &mut content::ZoneInstance,
    changed: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.label("危险区域");
        if zone.hazard.is_none() && ui.button("添加").clicked() {
            zone.hazard = Some(content::HazardRule {
                effects: vec![content::HazardEffect::new("oxygen", -2.0)],
                message: None,
            });
            if zone.zone_type == "Trigger" {
                zone.zone_type = "HazardZone".to_owned();
            }
            *changed = true;
        }
        if zone.hazard.is_some() && ui.button("清除").clicked() {
            zone.hazard = None;
            *changed = true;
        }
    });

    let Some(hazard) = &mut zone.hazard else {
        return;
    };

    let mut message = hazard.message.clone().unwrap_or_default();
    if labeled_text_edit(ui, "进入提示", &mut message) {
        set_optional_string(&mut hazard.message, message);
        *changed = true;
    }

    let mut remove_index = None;
    for (index, effect) in hazard.effects.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(format!("效果 #{}", index + 1));
            if labeled_text_edit(ui, "Meter", &mut effect.meter) {
                *changed = true;
            }
            *changed |= ui
                .add(
                    egui::DragValue::new(&mut effect.rate_per_second)
                        .speed(0.1)
                        .prefix("/秒 "),
                )
                .changed();
            if ui.button("删除").clicked() {
                remove_index = Some(index);
            }
        });
    }
    if let Some(index) = remove_index {
        hazard.effects.remove(index);
        *changed = true;
    }
    if ui.button("添加效果").clicked() {
        hazard
            .effects
            .push(content::HazardEffect::new("oxygen", -2.0));
        *changed = true;
    }
    ui.colored_label(
        THEME_MUTED_TEXT,
        "负数表示消耗，常用 meter：health / stamina / suit / oxygen / radiation / spores。",
    );
}

fn draw_zone_prompt_editor(
    ui: &mut egui::Ui,
    zone: &mut content::ZoneInstance,
    changed: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.label("提示区域");
        if zone.prompt.is_none() && ui.button("添加").clicked() {
            zone.prompt = Some(content::PromptRule::default());
            if zone.zone_type == "Trigger" {
                zone.zone_type = "PromptZone".to_owned();
            }
            *changed = true;
        }
        if zone.prompt.is_some() && ui.button("清除").clicked() {
            zone.prompt = None;
            *changed = true;
        }
    });

    let Some(prompt) = &mut zone.prompt else {
        return;
    };

    let mut message = prompt.message.clone().unwrap_or_default();
    if labeled_text_edit(ui, "屏幕提示", &mut message) {
        set_optional_string(&mut prompt.message, message);
        *changed = true;
    }
    let mut log_title = prompt.log_title.clone().unwrap_or_default();
    if labeled_text_edit(ui, "日志标题", &mut log_title) {
        set_optional_string(&mut prompt.log_title, log_title);
        *changed = true;
    }
    let mut log_detail = prompt.log_detail.clone().unwrap_or_default();
    if labeled_text_edit(ui, "日志内容", &mut log_detail) {
        set_optional_string(&mut prompt.log_detail, log_detail);
        *changed = true;
    }
    *changed |= ui
        .checkbox(&mut prompt.once, "只触发一次并写入存档")
        .changed();
}

fn draw_zone_objective_editor(
    ui: &mut egui::Ui,
    zone: &mut content::ZoneInstance,
    changed: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.label("任务目标");
        if zone.objective.is_none() && ui.button("添加").clicked() {
            zone.objective = Some(content::ObjectiveRule::default());
            if zone.zone_type == "Trigger" {
                zone.zone_type = "ObjectiveZone".to_owned();
            }
            *changed = true;
        }
        if zone.objective.is_some() && ui.button("清除").clicked() {
            zone.objective = None;
            *changed = true;
        }
    });

    let Some(objective) = &mut zone.objective else {
        return;
    };

    if labeled_text_edit(ui, "Objective ID", &mut objective.objective_id) {
        *changed = true;
    }
    let mut checkpoint_id = objective.checkpoint_id.clone().unwrap_or_default();
    if labeled_text_edit(ui, "Checkpoint ID", &mut checkpoint_id) {
        set_optional_string(&mut objective.checkpoint_id, checkpoint_id);
        *changed = true;
    }
    *changed |= ui
        .checkbox(&mut objective.complete_objective, "完成整个目标")
        .changed();
    let mut message = objective.message.clone().unwrap_or_default();
    if labeled_text_edit(ui, "屏幕提示", &mut message) {
        set_optional_string(&mut objective.message, message);
        *changed = true;
    }
    let mut log_title = objective.log_title.clone().unwrap_or_default();
    if labeled_text_edit(ui, "日志标题", &mut log_title) {
        set_optional_string(&mut objective.log_title, log_title);
        *changed = true;
    }
    let mut log_detail = objective.log_detail.clone().unwrap_or_default();
    if labeled_text_edit(ui, "日志内容", &mut log_detail) {
        set_optional_string(&mut objective.log_detail, log_detail);
        *changed = true;
    }
    *changed |= ui
        .checkbox(&mut objective.once, "只触发一次并写入存档")
        .changed();
    ui.colored_label(
        THEME_MUTED_TEXT,
        "ObjectiveZone 可启动目标；Checkpoint 填 Checkpoint ID 后推进目标步骤。",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_type_preset_initializes_required_payload() {
        let mut zone = test_zone("Trigger");

        assert!(apply_zone_type_preset(
            &mut zone,
            ZoneTypePreset::WalkSurface
        ));
        assert_eq!(zone.zone_type, "WalkSurface");
        assert!(zone.surface.is_some());

        assert!(apply_zone_type_preset(
            &mut zone,
            ZoneTypePreset::CollisionLine
        ));
        assert_eq!(zone.zone_type, "CollisionLine");
        assert!(zone.surface.is_none());

        assert!(apply_zone_type_preset(
            &mut zone,
            ZoneTypePreset::PromptZone
        ));
        assert_eq!(zone.zone_type, "PromptZone");
        assert!(zone.prompt.is_some());

        assert!(apply_zone_type_preset(
            &mut zone,
            ZoneTypePreset::Checkpoint
        ));
        assert_eq!(zone.zone_type, "Checkpoint");
        assert!(zone.objective.is_some());
    }

    #[test]
    fn zone_type_preset_current_checkbox_can_repair_missing_payload() {
        let mut zone = test_zone("WalkSurface");
        zone.surface = None;

        assert!(apply_zone_type_preset(
            &mut zone,
            ZoneTypePreset::WalkSurface
        ));
        assert_eq!(zone.zone_type, "WalkSurface");
        assert!(zone.surface.is_some());

        assert!(!apply_zone_type_preset(
            &mut zone,
            ZoneTypePreset::WalkSurface
        ));
    }

    fn test_zone(zone_type: &str) -> content::ZoneInstance {
        content::ZoneInstance {
            id: "zone_test".to_owned(),
            zone_type: zone_type.to_owned(),
            points: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            hazard: None,
            prompt: None,
            objective: None,
            surface: None,
            unlock: None,
            transition: None,
        }
    }
}
