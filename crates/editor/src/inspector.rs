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
        let mut next = self.document.clone();
        let mut changed = false;
        let mut next_selection = selection.clone();

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
                        &mut changed,
                    );
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
                        &mut changed,
                    );
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
    }
}
