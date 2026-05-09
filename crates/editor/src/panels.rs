use super::*;

impl EditorApp {
    fn draw_asset_panel(&mut self, ui: &mut egui::Ui) {
        ui.small(format!("{} 个 metadata 素材", self.registry.assets().len()));
        search_field(ui, &mut self.asset_search, "搜索 id / tag / path");
        let search = self.asset_search.to_ascii_lowercase();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let categories = self
                    .registry
                    .categories()
                    .into_iter()
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
                for category in categories {
                    egui::CollapsingHeader::new(category_label(&category))
                        .default_open(category == "tiles" || category == "props")
                        .show(ui, |ui| {
                            let assets = self
                                .registry
                                .in_category(&category)
                                .filter(|asset| {
                                    search.is_empty() || asset_matches_search(asset, &search)
                                })
                                .cloned()
                                .collect::<Vec<_>>();

                            self.draw_asset_grid(ui, &category, &assets);
                        });
                }
            });
    }

    fn draw_asset_grid(&mut self, ui: &mut egui::Ui, category: &str, assets: &[AssetEntry]) {
        let columns = asset_grid_columns(ui);
        egui::Grid::new(format!("asset_grid_{category}"))
            .num_columns(columns)
            .spacing(vec2(6.0, 6.0))
            .show(ui, |ui| {
                for (index, asset) in assets.iter().enumerate() {
                    let selected = self.selected_asset.as_deref() == Some(asset.id.as_str());
                    let response = asset_tile(
                        ui,
                        selected,
                        &compact_asset_label(&asset.id),
                        self.thumbnails.get(&asset.id),
                    );
                    if response.clicked() {
                        self.select_asset(asset);
                    }
                    if selected || response.rect.intersects(ui.clip_rect()) {
                        self.request_asset_texture(&asset.id);
                    }
                    response.on_hover_text(&asset.id);

                    if (index + 1) % columns == 0 {
                        ui.end_row();
                    }
                }
            });
    }

    fn select_asset(&mut self, asset: &AssetEntry) {
        self.selected_asset = Some(asset.id.clone());
        self.request_asset_texture(&asset.id);
        self.clear_selection();
        self.active_layer = asset.default_layer;
        self.tool = ToolKind::Brush;
        if asset.kind == AssetKind::Tile {
            let footprint = self.asset_tile_footprint(asset);
            self.ground_footprint_w = footprint[0];
            self.ground_footprint_h = footprint[1];
        }
        self.status = format!("Selected {}", asset.id);
    }

    pub(super) fn draw_left_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            EditorTabs::default().show(ui, &mut self.active_left_tab, LeftSidebarTab::ALL, |tab| {
                tab.label()
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if editor_icon_button(ui, "<", "收起左侧栏").clicked() {
                    self.show_left_sidebar = false;
                }
            });
        });
        ui.separator();

        match self.active_left_tab {
            LeftSidebarTab::Assets => {
                self.draw_asset_panel(ui);
            }
            LeftSidebarTab::Layers => {
                self.draw_layer_panel(ui);
            }
            LeftSidebarTab::Outliner => {
                egui::ScrollArea::vertical().show(ui, |ui| self.draw_outliner_panel(ui));
            }
        }
    }

    fn draw_layer_panel(&mut self, ui: &mut egui::Ui) {
        for layer in LayerKind::ALL {
            let count = self.layer_item_count(layer);
            let state = self.layer_states.entry(layer).or_default();
            let mut visible = state.visible;
            let mut locked = state.locked;
            let response = layer_row(
                ui,
                self.active_layer == layer,
                layer.zh_label(),
                count,
                &mut visible,
                &mut locked,
            );
            let state = self.layer_states.entry(layer).or_default();
            state.visible = visible;
            state.locked = locked;
            if response.selected_clicked {
                self.active_layer = layer;
            }
        }
    }

    fn draw_outliner_panel(&mut self, ui: &mut egui::Ui) {
        search_field(
            ui,
            &mut self.outliner_search,
            "搜索 id / asset / type / codex / tag",
        );

        let search = self.outliner_search.trim().to_ascii_lowercase();
        let entries = self.outliner_entries();
        let visible_entries = entries
            .into_iter()
            .filter(|entry| outliner_matches(entry, &search))
            .collect::<Vec<_>>();

        let scan_count = visible_entries
            .iter()
            .filter(|entry| entry.badges.iter().any(|badge| badge.label == "scan"))
            .count();
        ui.small(format!(
            "{} 个对象，{} 个扫描候选",
            visible_entries.len(),
            scan_count
        ));

        for group in OUTLINER_GROUPS {
            let group_entries = visible_entries
                .iter()
                .filter(|entry| entry.group == *group)
                .cloned()
                .collect::<Vec<_>>();
            if group_entries.is_empty() {
                continue;
            }

            egui::CollapsingHeader::new(format!("{group} ({})", group_entries.len()))
                .default_open(matches!(*group, "Entities" | "Objects" | "Zones"))
                .show(ui, |ui| {
                    for entry in group_entries {
                        self.draw_outliner_row(ui, entry);
                    }
                });
        }
    }

    fn draw_outliner_row(&mut self, ui: &mut egui::Ui, entry: OutlinerEntry) {
        let selected = entry.selection.as_ref().is_some_and(|selection| {
            self.current_selection_list()
                .iter()
                .any(|current| current == selection)
        });
        let response = tree_row(
            ui,
            selected,
            &entry.label,
            &entry.detail,
            entry.badges.iter().map(|badge| TreeBadge {
                label: badge.label,
                color: badge.color,
            }),
        );

        if response.clicked() {
            self.focus_outliner_entry(&entry);
        }
        response.on_hover_text(&entry.search_text);
    }

    fn focus_outliner_entry(&mut self, entry: &OutlinerEntry) {
        if let Some(selection) = entry.selection.clone() {
            self.active_layer = selection.layer;
            self.selected_asset = None;
            self.set_single_selection(Some(selection.clone()));
            self.status = format!("已定位 {}", selection.label());
        } else {
            self.clear_selection();
            self.status = format!("已定位 {}", entry.label);
        }
        self.pending_focus_world = Some(entry.focus_world);
    }

    fn outliner_entries(&self) -> Vec<OutlinerEntry> {
        let mut entries = Vec::new();
        let tile_size = self.document.tile_size as f32;
        let solid_cells = self
            .document
            .layers
            .collision
            .iter()
            .filter(|cell| cell.solid)
            .map(|cell| (cell.x, cell.y))
            .collect::<BTreeSet<_>>();
        let codex_counts = self.entity_codex_counts();

        for spawn in &self.document.spawns {
            let cell = (spawn.x.floor() as i32, spawn.y.floor() as i32);
            let mut badges = Vec::new();
            if solid_cells.contains(&cell) {
                badges.push(OutlinerBadge {
                    label: "solid",
                    color: THEME_ERROR,
                });
            }
            entries.push(outliner_entry(
                "Spawns",
                format!("spawn {}", spawn.id),
                format!("{:.1}, {:.1}", spawn.x, spawn.y),
                None,
                vec2(spawn.x * tile_size, spawn.y * tile_size),
                badges,
                [spawn.id.as_str(), "spawn"].join(" "),
            ));
        }

        for entity in &self.document.layers.entities {
            let asset = self.registry.get(&entity.asset);
            let mut badges = Vec::new();
            let codex_id = asset.and_then(|asset| asset.codex_id.as_deref());
            if codex_id.is_some() {
                badges.push(OutlinerBadge {
                    label: "scan",
                    color: THEME_ACCENT_STRONG,
                });
                if entity.interaction_rect.is_none() {
                    badges.push(OutlinerBadge {
                        label: "missing rect",
                        color: THEME_WARNING,
                    });
                }
            }
            if let Some(codex_id) = codex_id {
                badges.extend(self.codex_status_badges(codex_id));
                if codex_counts.get(codex_id).copied().unwrap_or(0) > 1 {
                    badges.push(OutlinerBadge {
                        label: "dup codex",
                        color: THEME_WARNING,
                    });
                }
            }
            if entity
                .unlock
                .as_ref()
                .is_some_and(|unlock| !unlock.is_empty())
            {
                badges.push(OutlinerBadge {
                    label: "unlock",
                    color: THEME_WARNING,
                });
            }
            if entity.entity_type.trim().is_empty() {
                badges.push(OutlinerBadge {
                    label: "no type",
                    color: THEME_ERROR,
                });
            }
            if asset.is_none() {
                badges.push(OutlinerBadge {
                    label: "missing asset",
                    color: THEME_ERROR,
                });
            }

            let focus_world = asset
                .map(|asset| anchor_grid_to_world(tile_size, entity.x, entity.y, asset.anchor))
                .unwrap_or_else(|| vec2(entity.x * tile_size, entity.y * tile_size));
            let tags = asset.map(|asset| asset.tags.join(" ")).unwrap_or_default();
            let codex_search = codex_id
                .map(|codex_id| self.codex_search_text(codex_id))
                .unwrap_or_default();
            let unlock_search = unlock_search_text(entity.unlock.as_ref());
            entries.push(outliner_entry(
                "Entities",
                entity.id.clone(),
                format!("{} | {}", entity.asset, entity.entity_type),
                Some(SelectedItem {
                    layer: LayerKind::Entities,
                    id: entity.id.clone(),
                }),
                focus_world,
                badges,
                [
                    entity.id.as_str(),
                    entity.asset.as_str(),
                    entity.entity_type.as_str(),
                    codex_id.unwrap_or_default(),
                    tags.as_str(),
                    codex_search.as_str(),
                    unlock_search.as_str(),
                ]
                .join(" "),
            ));
        }

        for object in &self.document.layers.objects {
            let asset = self.registry.get(&object.asset);
            let mut badges = Vec::new();
            let codex_id = asset.and_then(|asset| asset.codex_id.as_deref());
            if codex_id.is_some() {
                badges.push(OutlinerBadge {
                    label: "codex only",
                    color: THEME_WARNING,
                });
                if let Some(codex_id) = codex_id {
                    badges.extend(self.codex_status_badges(codex_id));
                }
            }
            if asset.is_none() {
                badges.push(OutlinerBadge {
                    label: "missing asset",
                    color: THEME_ERROR,
                });
            }
            entries.push(self.object_outliner_entry(
                "Objects",
                LayerKind::Objects,
                object,
                asset,
                codex_id,
                badges,
            ));
        }

        for decal in &self.document.layers.decals {
            let asset = self.registry.get(&decal.asset);
            let mut badges = Vec::new();
            let codex_id = asset.and_then(|asset| asset.codex_id.as_deref());
            if let Some(codex_id) = codex_id {
                badges.push(OutlinerBadge {
                    label: "codex only",
                    color: THEME_WARNING,
                });
                badges.extend(self.codex_status_badges(codex_id));
            }
            if asset.is_none() {
                badges.push(OutlinerBadge {
                    label: "missing asset",
                    color: THEME_ERROR,
                });
            }
            entries.push(self.object_outliner_entry(
                "Decals",
                LayerKind::Decals,
                decal,
                asset,
                codex_id,
                badges,
            ));
        }

        for zone in &self.document.layers.zones {
            let mut badges = Vec::new();
            if zone.zone_type.trim().is_empty() {
                badges.push(OutlinerBadge {
                    label: "no type",
                    color: THEME_ERROR,
                });
            } else if !EDITOR_KNOWN_ZONE_TYPES.contains(&zone.zone_type.as_str()) {
                badges.push(OutlinerBadge {
                    label: "unknown",
                    color: THEME_WARNING,
                });
            }
            if zone.points.len() < 3 {
                badges.push(OutlinerBadge {
                    label: "few points",
                    color: THEME_WARNING,
                });
            }
            if zone.surface.is_some() {
                badges.push(OutlinerBadge {
                    label: "surface",
                    color: THEME_ACCENT_STRONG,
                });
            }
            if zone
                .unlock
                .as_ref()
                .is_some_and(|unlock| !unlock.is_empty())
            {
                badges.push(OutlinerBadge {
                    label: "unlock",
                    color: THEME_WARNING,
                });
            }
            let unlock_search = unlock_search_text(zone.unlock.as_ref());
            entries.push(outliner_entry(
                "Zones",
                zone.id.clone(),
                format!("{} | {} points", zone.zone_type, zone.points.len()),
                Some(SelectedItem {
                    layer: LayerKind::Zones,
                    id: zone.id.clone(),
                }),
                zone_focus_world(zone, tile_size),
                badges,
                [
                    zone.id.as_str(),
                    zone.zone_type.as_str(),
                    unlock_search.as_str(),
                ]
                .join(" "),
            ));
        }

        for tile in &self.document.layers.ground {
            entries.push(outliner_entry(
                "Ground",
                ground_selection_id(tile.x, tile.y),
                format!("{} | {}x{}", tile.asset, tile.w.max(1), tile.h.max(1)),
                Some(SelectedItem {
                    layer: LayerKind::Ground,
                    id: ground_selection_id(tile.x, tile.y),
                }),
                vec2(
                    (tile.x as f32 + tile.w.max(1) as f32 * 0.5) * tile_size,
                    (tile.y as f32 + tile.h.max(1) as f32 * 0.5) * tile_size,
                ),
                Vec::new(),
                [tile.asset.as_str(), &ground_selection_id(tile.x, tile.y)].join(" "),
            ));
        }

        entries
    }

    fn object_outliner_entry(
        &self,
        group: &'static str,
        layer: LayerKind,
        instance: &content::ObjectInstance,
        asset: Option<&AssetEntry>,
        codex_id: Option<&str>,
        badges: Vec<OutlinerBadge>,
    ) -> OutlinerEntry {
        let tile_size = self.document.tile_size as f32;
        let focus_world = asset
            .map(|asset| anchor_grid_to_world(tile_size, instance.x, instance.y, asset.anchor))
            .unwrap_or_else(|| vec2(instance.x * tile_size, instance.y * tile_size));
        let tags = asset.map(|asset| asset.tags.join(" ")).unwrap_or_default();
        let codex_search = codex_id
            .map(|codex_id| self.codex_search_text(codex_id))
            .unwrap_or_default();
        outliner_entry(
            group,
            instance.id.clone(),
            instance.asset.clone(),
            Some(SelectedItem {
                layer,
                id: instance.id.clone(),
            }),
            focus_world,
            badges,
            [
                instance.id.as_str(),
                instance.asset.as_str(),
                codex_id.unwrap_or_default(),
                tags.as_str(),
                codex_search.as_str(),
            ]
            .join(" "),
        )
    }

    fn codex_status_badges(&self, codex_id: &str) -> Vec<OutlinerBadge> {
        let Some(database) = &self.codex_database else {
            return vec![OutlinerBadge {
                label: "no codex db",
                color: THEME_WARNING,
            }];
        };
        let Some(entry) = database.get(codex_id) else {
            return vec![OutlinerBadge {
                label: "missing codex",
                color: THEME_ERROR,
            }];
        };

        let mut badges = Vec::new();
        if entry.title.trim().is_empty() {
            badges.push(OutlinerBadge {
                label: "no title",
                color: THEME_WARNING,
            });
        }
        if entry.category.trim().is_empty() {
            badges.push(OutlinerBadge {
                label: "no category",
                color: THEME_WARNING,
            });
        }
        if entry.description.trim().is_empty() {
            badges.push(OutlinerBadge {
                label: "no text",
                color: THEME_WARNING,
            });
        }
        badges
    }

    fn codex_search_text(&self, codex_id: &str) -> String {
        self.codex_database
            .as_ref()
            .and_then(|database| database.get(codex_id))
            .map(|entry| format!("{} {} {}", entry.title, entry.category, entry.description))
            .unwrap_or_default()
    }

    fn entity_codex_counts(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for entity in &self.document.layers.entities {
            let Some(codex_id) = self
                .registry
                .get(&entity.asset)
                .and_then(|asset| asset.codex_id.as_ref())
            else {
                continue;
            };
            *counts.entry(codex_id.clone()).or_insert(0) += 1;
        }
        counts
    }

    fn layer_item_count(&self, layer: LayerKind) -> usize {
        match layer {
            LayerKind::Ground => self.document.layers.ground.len(),
            LayerKind::Decals => self.document.layers.decals.len(),
            LayerKind::Objects => self.document.layers.objects.len(),
            LayerKind::Entities => self.document.layers.entities.len(),
            LayerKind::Zones => self.document.layers.zones.len(),
            LayerKind::Collision => self.document.layers.collision.len(),
        }
    }
}
