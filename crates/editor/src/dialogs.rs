use super::*;

impl EditorApp {
    pub(super) fn draw_dialogs(&mut self, ctx: &EguiContext) {
        if self.show_new_map_dialog {
            egui::Window::new("新建地图")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    labeled_text_edit(ui, "地图 ID", &mut self.new_map_draft.id);
                    labeled_text_edit(ui, "模式", &mut self.new_map_draft.mode);
                    ui.add(
                        egui::DragValue::new(&mut self.new_map_draft.width)
                            .range(1..=512)
                            .prefix("宽 "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.new_map_draft.height)
                            .range(1..=512)
                            .prefix("高 "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.new_map_draft.tile_size)
                            .range(1..=256)
                            .prefix("格 "),
                    );
                    ui.separator();
                    ui.label("出生点");
                    labeled_text_edit(ui, "出生点 ID", &mut self.new_map_draft.spawn_id);
                    ui.add(
                        egui::DragValue::new(&mut self.new_map_draft.spawn_x)
                            .speed(0.1)
                            .prefix("x "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.new_map_draft.spawn_y)
                            .speed(0.1)
                            .prefix("y "),
                    );
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("创建").clicked() {
                            self.create_new_map_from_draft();
                        }
                        if ui.button("取消").clicked() {
                            self.show_new_map_dialog = false;
                        }
                    });
                });
        }

        if let Some(recovery) = self.autosave_recovery.clone() {
            egui::Window::new("发现自动保存")
                .collapsible(false)
                .resizable(false)
                .default_width(520.0)
                .show(ctx, |ui| {
                    ui.label("检测到比主文件更新的自动保存。");
                    ui.separator();
                    ui.label("主文件");
                    ui.monospace(display_project_path(&self.project_root, &recovery.map_path));
                    ui.label("自动保存");
                    ui.monospace(display_project_path(
                        &self.project_root,
                        &recovery.autosave_path,
                    ));
                    ui.label(format!(
                        "自动保存比主文件新 {}",
                        format_duration(recovery.newer_by)
                    ));
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("恢复自动保存").clicked() {
                            self.restore_autosave(recovery.clone());
                        }
                        if ui.button("丢弃自动保存").clicked() {
                            self.discard_autosave(recovery.clone());
                        }
                        if ui.button("稍后").clicked() {
                            self.autosave_recovery = None;
                            self.status = "已暂时忽略自动保存，文件仍保留".to_owned();
                        }
                    });
                });
        }

        if let Some(path) = self.open_confirm_path.clone() {
            egui::Window::new("未保存的修改")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "当前地图 {} 有未保存修改。",
                        display_project_path(&self.project_root, &self.map_path)
                    ));
                    ui.horizontal(|ui| {
                        if ui.button("保存并打开").clicked() {
                            self.save_map();
                            if !self.dirty {
                                self.open_map(path.clone());
                            }
                        }
                        if ui.button("放弃修改").clicked() {
                            self.open_map(path.clone());
                        }
                        if ui.button("取消").clicked() {
                            self.open_confirm_path = None;
                            self.pending_open_focus_spawn = None;
                        }
                    });
                });
        }

        if let Some(path) = self.delete_confirm_path.clone() {
            egui::Window::new("删除地图")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "确定删除 {}？",
                        display_project_path(&self.project_root, &path)
                    ));
                    ui.horizontal(|ui| {
                        if ui.button("删除").clicked() {
                            self.delete_map_file(path.clone());
                        }
                        if ui.button("取消").clicked() {
                            self.delete_confirm_path = None;
                        }
                    });
                });
        }

        if self.show_validation_panel {
            egui::Window::new("地图校验")
                .default_width(420.0)
                .show(ctx, |ui| {
                    ui.label(validation_summary(&self.validation_issues));
                    ui.separator();
                    if self.validation_issues.is_empty() {
                        ui.label("没有问题");
                    }
                    for issue in &self.validation_issues {
                        let color = match issue.severity {
                            MapValidationSeverity::Error => THEME_ERROR,
                            MapValidationSeverity::Warning => THEME_WARNING,
                        };
                        ui.colored_label(color, &issue.message);
                    }
                    if ui.button("关闭").clicked() {
                        self.show_validation_panel = false;
                    }
                });
        }

        if self.show_asset_dependency_report {
            let report = self.asset_dependency_report.clone();
            egui::Window::new("资产依赖报告")
                .default_width(760.0)
                .default_height(560.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("当前地图");
                        ui.monospace(display_project_path(&self.project_root, &self.map_path));
                    });
                    ui.label(report.summary());
                    ui.horizontal(|ui| {
                        if ui.button("刷新").clicked() {
                            self.open_asset_dependency_report();
                        }
                        if ui.button("打开未登记图片").clicked() {
                            self.show_unregistered_assets = true;
                        }
                        if ui.button("关闭").clicked() {
                            self.show_asset_dependency_report = false;
                        }
                    });
                    ui.separator();
                    if report.item_count() == 0 {
                        ui.label("没有发现资产依赖问题。");
                    } else {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            draw_asset_catalog_section(
                                ui,
                                "缺文件",
                                &report.missing_files,
                                &self.project_root,
                            );
                            draw_string_section(ui, "未登记 PNG", &report.unregistered_pngs);
                            draw_asset_reference_section(
                                ui,
                                "地图引用未知素材",
                                &report.unknown_references,
                            );
                            draw_asset_catalog_section(
                                ui,
                                "地图未使用素材",
                                &report.unused_assets,
                                &self.project_root,
                            );
                        });
                    }
                });
        }

        if self.show_asset_dialog {
            egui::Window::new(if self.asset_editing_id.is_some() {
                "编辑素材"
            } else {
                "添加素材"
            })
            .default_width(460.0)
            .show(ctx, |ui| {
                self.draw_asset_draft_editor(ui, ctx);
            });
        }

        if self.show_unregistered_assets {
            egui::Window::new("未登记图片")
                .default_width(620.0)
                .default_height(520.0)
                .show(ctx, |ui| {
                    ui.label(
                        "这里扫描 PNG 只是为了辅助登记，未登记图片不会自动进入游戏或地图编辑。",
                    );
                    ui.horizontal(|ui| {
                        ui.label("扫描文件夹");
                        ui.monospace(display_project_path(
                            &self.project_root,
                            &self.asset_scan_root,
                        ));
                        if ui.button("选择文件夹").clicked() {
                            self.pick_asset_scan_folder();
                        }
                        if ui.button("默认").clicked() {
                            self.asset_scan_root = self.project_root.join("assets").join("sprites");
                        }
                        if ui.button("登记全部").clicked() {
                            let mut paths = Vec::new();
                            collect_png_paths(&self.asset_scan_root, &mut paths);
                            self.add_asset_definitions_from_paths(paths, ctx);
                        }
                    });
                    ui.separator();
                    let unregistered = self.unregistered_sprite_paths();
                    if unregistered.is_empty() {
                        ui.label("没有发现未登记 PNG。");
                    } else {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for path in unregistered {
                                ui.horizontal(|ui| {
                                    ui.monospace(&path);
                                    if ui.button("登记").clicked() {
                                        self.fill_asset_draft_from_path(&path);
                                    }
                                });
                            }
                        });
                    }
                    ui.separator();
                    if ui.button("关闭").clicked() {
                        self.show_unregistered_assets = false;
                    }
                });
        }
    }

    fn draw_asset_draft_editor(&mut self, ui: &mut egui::Ui, ctx: &EguiContext) {
        let entity_type_options = self.entity_type_options();
        let codex_id_options = self.codex_id_options();

        ui.label("素材 id / 路径");
        labeled_text_edit(ui, "素材 ID", &mut self.asset_draft.id);
        labeled_text_edit(ui, "图片路径", &mut self.asset_draft.path);
        ui.horizontal(|ui| {
            if ui.button("选择 PNG 文件").clicked() {
                self.pick_asset_file_into_draft(ctx);
            }
            if ui.button("批量选择文件").clicked() {
                self.pick_and_add_asset_files(ctx);
            }
            if ui.button("批量选择文件夹").clicked() {
                self.pick_and_add_asset_folder(ctx);
            }
            if ui.button("从图片尺寸读取").clicked() {
                match image_dimensions(&self.project_root.join(&self.asset_draft.path)) {
                    Some([width, height]) => {
                        self.asset_draft.default_size = [width, height];
                        if self.asset_draft.kind == AssetKind::Tile {
                            self.asset_draft.footprint =
                                infer_tile_footprint([width, height], self.document.tile_size)
                                    .unwrap_or(self.asset_draft.footprint);
                        }
                        self.status = "已读取图片尺寸".to_owned();
                    }
                    None => {
                        self.status = "图片尺寸读取失败".to_owned();
                    }
                }
            }
            if ui.button("按路径推断").clicked() {
                let path = self.asset_draft.path.clone();
                if path.trim().is_empty() {
                    self.status = "请先填写图片路径".to_owned();
                } else {
                    self.asset_draft = infer_asset_draft_from_path(&self.project_root, &path);
                }
            }
        });

        ui.separator();
        ui.label("分类 / 类型");
        labeled_text_edit(ui, "分类", &mut self.asset_draft.category);
        egui::ComboBox::from_id_salt("asset_draft_kind")
            .selected_text(self.asset_draft.kind.zh_label())
            .show_ui(ui, |ui| {
                for kind in AssetKind::ALL {
                    if ui
                        .selectable_value(&mut self.asset_draft.kind, kind, kind.zh_label())
                        .clicked()
                    {
                        apply_kind_defaults(&mut self.asset_draft);
                    }
                }
            });
        egui::ComboBox::from_id_salt("asset_draft_layer")
            .selected_text(self.asset_draft.default_layer.zh_label())
            .show_ui(ui, |ui| {
                for layer in LayerKind::ALL {
                    ui.selectable_value(
                        &mut self.asset_draft.default_layer,
                        layer,
                        layer.zh_label(),
                    );
                }
            });

        ui.separator();
        ui.label("默认尺寸 / 放置规则");
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.asset_draft.default_size[0])
                    .range(1.0..=4096.0)
                    .speed(1.0)
                    .prefix("宽 "),
            );
            ui.add(
                egui::DragValue::new(&mut self.asset_draft.default_size[1])
                    .range(1.0..=4096.0)
                    .speed(1.0)
                    .prefix("高 "),
            );
        });
        if self.asset_draft.kind == AssetKind::Tile {
            if let Some(footprint) =
                infer_tile_footprint(self.asset_draft.default_size, self.document.tile_size)
            {
                self.asset_draft.footprint = footprint;
            }
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut self.asset_draft.footprint[0])
                        .range(1..=64)
                        .speed(0.1)
                        .prefix("占格宽 "),
                );
                ui.add(
                    egui::DragValue::new(&mut self.asset_draft.footprint[1])
                        .range(1..=64)
                        .speed(0.1)
                        .prefix("占格高 "),
                );
            });
            if infer_tile_footprint(self.asset_draft.default_size, self.document.tile_size)
                .is_none()
            {
                ui.colored_label(
                    THEME_WARNING,
                    "尺寸不是当前地图格子的整数倍，请手动确认占格",
                );
            }
        }
        egui::ComboBox::from_id_salt("asset_draft_anchor")
            .selected_text(anchor_label(self.asset_draft.anchor))
            .show_ui(ui, |ui| {
                for anchor in [
                    AnchorKind::TopLeft,
                    AnchorKind::Center,
                    AnchorKind::BottomCenter,
                ] {
                    ui.selectable_value(&mut self.asset_draft.anchor, anchor, anchor_label(anchor));
                }
            });
        egui::ComboBox::from_id_salt("asset_draft_snap")
            .selected_text(snap_label(self.asset_draft.snap))
            .show_ui(ui, |ui| {
                for snap in [SnapMode::Grid, SnapMode::HalfGrid, SnapMode::Free] {
                    ui.selectable_value(&mut self.asset_draft.snap, snap, snap_label(snap));
                }
            });

        if matches!(
            self.asset_draft.kind,
            AssetKind::Tile | AssetKind::Object | AssetKind::Entity
        ) {
            let mut changed = false;
            entity_rect_editor(
                ui,
                "默认碰撞模板",
                &mut self.asset_draft.default_collision_rect,
                &mut changed,
            );
        }
        if matches!(self.asset_draft.kind, AssetKind::Object | AssetKind::Entity) {
            let mut changed = false;
            entity_rect_editor(
                ui,
                "默认遮挡/排序模板",
                &mut self.asset_draft.default_depth_rect,
                &mut changed,
            );
        }
        if self.asset_draft.kind == AssetKind::Entity {
            let mut changed = false;
            entity_rect_editor(
                ui,
                "默认交互模板",
                &mut self.asset_draft.default_interaction_rect,
                &mut changed,
            );
        }

        ui.separator();
        ui.label("标签 / 额外属性");
        labeled_text_edit(ui, "标签", &mut self.asset_draft.tags);
        labeled_text_edit_with_options(
            ui,
            "实体类型",
            "asset_draft_entity_type",
            &mut self.asset_draft.entity_type,
            &entity_type_options,
        );
        labeled_text_edit_with_options(
            ui,
            "图鉴 ID",
            "asset_draft_codex_id",
            &mut self.asset_draft.codex_id,
            &codex_id_options,
        );
        draw_asset_draft_scan_status(ui, &self.asset_draft, self.codex_database.as_ref());

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("应用").clicked() {
                self.apply_asset_draft(ctx);
            }
            if ui.button("应用并保存").clicked() {
                self.apply_asset_draft(ctx);
                if self.asset_db_dirty {
                    self.save_asset_database();
                }
            }
            if ui.button("取消").clicked() {
                self.show_asset_dialog = false;
            }
        });
    }

    fn create_new_map_from_draft(&mut self) {
        let Some(id) = sanitize_map_id(&self.new_map_draft.id) else {
            self.status = "新建失败：地图 id 为空".to_owned();
            return;
        };
        let id = unique_map_id(&self.project_root, &id);
        self.document = MapDocument {
            id: id.clone(),
            mode: self.new_map_draft.mode.clone(),
            tile_size: self.new_map_draft.tile_size.max(1),
            width: self.new_map_draft.width.max(1),
            height: self.new_map_draft.height.max(1),
            layers: Default::default(),
            spawns: vec![content::SpawnPoint {
                id: self.new_map_draft.spawn_id.clone(),
                x: self.new_map_draft.spawn_x,
                y: self.new_map_draft.spawn_y,
            }],
        };
        self.map_path = maps_dir(&self.project_root).join(format!("{id}.ron"));
        self.selected_map_path = self.map_path.clone();
        self.save_as_id = id;
        self.clear_selection();
        self.hidden_items.clear();
        self.selected_asset = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.dirty = true;
        self.show_new_map_dialog = false;
        self.autosave_recovery = None;
        self.status = "已创建新地图".to_owned();
    }
}

fn draw_asset_catalog_section(
    ui: &mut egui::Ui,
    title: &str,
    entries: &[AssetCatalogEntry],
    project_root: &std::path::Path,
) {
    egui::CollapsingHeader::new(format!("{title} ({})", entries.len()))
        .default_open(!entries.is_empty())
        .show(ui, |ui| {
            if entries.is_empty() {
                ui.label("无");
                return;
            }
            for entry in entries {
                ui.horizontal_wrapped(|ui| {
                    ui.monospace(&entry.asset_id);
                    ui.label(&entry.category);
                    ui.monospace(display_report_asset_path(project_root, &entry.path));
                });
            }
        });
}

fn draw_asset_reference_section(ui: &mut egui::Ui, title: &str, entries: &[AssetReferenceIssue]) {
    egui::CollapsingHeader::new(format!("{title} ({})", entries.len()))
        .default_open(!entries.is_empty())
        .show(ui, |ui| {
            if entries.is_empty() {
                ui.label("无");
                return;
            }
            for entry in entries {
                ui.horizontal_wrapped(|ui| {
                    ui.label(entry.layer.zh_label());
                    ui.monospace(&entry.owner);
                    ui.monospace(&entry.asset_id);
                });
            }
        });
}

fn draw_string_section(ui: &mut egui::Ui, title: &str, entries: &[String]) {
    egui::CollapsingHeader::new(format!("{title} ({})", entries.len()))
        .default_open(!entries.is_empty())
        .show(ui, |ui| {
            if entries.is_empty() {
                ui.label("无");
                return;
            }
            for entry in entries {
                ui.monospace(entry);
            }
        });
}

fn display_report_asset_path(project_root: &std::path::Path, path: &std::path::Path) -> String {
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    };
    display_project_path(project_root, &absolute_path)
}

fn format_duration(duration: std::time::Duration) -> String {
    let seconds = duration.as_secs();
    if seconds >= 3600 {
        format!("{} 小时 {} 分钟", seconds / 3600, seconds % 3600 / 60)
    } else if seconds >= 60 {
        format!("{} 分钟 {} 秒", seconds / 60, seconds % 60)
    } else {
        format!("{seconds} 秒")
    }
}
