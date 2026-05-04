mod asset_registry;
mod map_document;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use asset_registry::{AssetEntry, AssetKind, AssetRegistry};
use eframe::egui::{
    self, Color32, Context as EguiContext, Key, Modifiers, Pos2, Rect, Sense, Stroke,
    TextureHandle, TextureOptions, Vec2, vec2,
};
use map_document::{DEFAULT_MAP_PATH, LayerKind, MapDocument};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 760.0])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Alien Archive Overworld Map Editor",
        options,
        Box::new(|cc| Ok(Box::new(EditorApp::new(cc)))),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ToolKind {
    Select,
    PaintTile,
    PlaceObject,
    Erase,
    Pan,
}

impl ToolKind {
    const ALL: [Self; 5] = [
        Self::Select,
        Self::PaintTile,
        Self::PlaceObject,
        Self::Erase,
        Self::Pan,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::PaintTile => "Paint Tile",
            Self::PlaceObject => "Place Object",
            Self::Erase => "Erase",
            Self::Pan => "Pan",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SelectedItem {
    layer: LayerKind,
    id: String,
}

impl SelectedItem {
    fn label(&self) -> String {
        format!("{}:{}", self.layer.label(), self.id)
    }
}

struct EditorApp {
    project_root: PathBuf,
    map_path: PathBuf,
    registry: AssetRegistry,
    document: MapDocument,
    selected_asset: Option<String>,
    selected_item: Option<SelectedItem>,
    active_layer: LayerKind,
    tool: ToolKind,
    show_grid: bool,
    show_collision: bool,
    show_entity_bounds: bool,
    pan: Vec2,
    zoom: f32,
    mouse_tile: Option<[i32; 2]>,
    thumbnails: HashMap<String, TextureHandle>,
    status: String,
}

impl EditorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let project_root = project_root();
        let map_path = project_root.join(DEFAULT_MAP_PATH);
        let registry = AssetRegistry::scan(&project_root).unwrap_or_else(|error| {
            eprintln!("asset scan failed: {error:?}");
            AssetRegistry::default()
        });
        let document =
            MapDocument::load(&map_path).unwrap_or_else(|_| MapDocument::new_landing_site());
        let mut app = Self {
            project_root,
            map_path,
            registry,
            document,
            selected_asset: None,
            selected_item: None,
            active_layer: LayerKind::Ground,
            tool: ToolKind::PaintTile,
            show_grid: true,
            show_collision: true,
            show_entity_bounds: true,
            pan: vec2(48.0, 48.0),
            zoom: 1.0,
            mouse_tile: None,
            thumbnails: HashMap::new(),
            status: "Ready".to_owned(),
        };

        app.load_visible_textures(&cc.egui_ctx);
        app
    }

    fn load_visible_textures(&mut self, ctx: &EguiContext) {
        for asset in self.registry.assets() {
            if self.thumbnails.contains_key(&asset.id) {
                continue;
            }

            match load_thumbnail(ctx, asset) {
                Ok(texture) => {
                    self.thumbnails.insert(asset.id.clone(), texture);
                }
                Err(error) => {
                    eprintln!(
                        "failed to load thumbnail {}: {error:?}",
                        asset.relative_path
                    );
                }
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &EguiContext) {
        ctx.input_mut(|input| {
            if input.consume_key(Modifiers::CTRL, Key::S) {
                self.save_map();
            }
            if input.key_pressed(Key::Num1) {
                self.tool = ToolKind::Select;
            }
            if input.key_pressed(Key::Num2) {
                self.tool = ToolKind::PaintTile;
            }
            if input.key_pressed(Key::Num3) {
                self.tool = ToolKind::PlaceObject;
            }
            if input.key_pressed(Key::Num4) {
                self.tool = ToolKind::Erase;
            }
        });
    }

    fn save_map(&mut self) {
        match self.document.save(&self.map_path) {
            Ok(()) => {
                self.status = format!(
                    "Saved {}",
                    display_project_path(&self.project_root, &self.map_path)
                );
            }
            Err(error) => {
                self.status = format!("Save failed: {error:#}");
            }
        }
    }

    fn selected_asset(&self) -> Option<&AssetEntry> {
        self.selected_asset
            .as_deref()
            .and_then(|id| self.registry.get(id))
    }

    fn draw_top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("New").clicked() {
                self.document = MapDocument::new_landing_site();
                self.selected_item = None;
                self.status = "New overworld_landing_site map".to_owned();
            }
            if ui.button("Save RON").clicked() {
                self.save_map();
            }

            ui.separator();
            ui.label("Tool");
            for tool in ToolKind::ALL {
                ui.selectable_value(&mut self.tool, tool, tool.label());
            }

            ui.separator();
            ui.label("Layer");
            egui::ComboBox::from_id_source("active_layer")
                .selected_text(self.active_layer.label())
                .show_ui(ui, |ui| {
                    for layer in LayerKind::ALL {
                        ui.selectable_value(&mut self.active_layer, layer, layer.label());
                    }
                });

            ui.separator();
            ui.checkbox(&mut self.show_grid, "Grid");
            ui.checkbox(&mut self.show_collision, "Collision");
            ui.checkbox(&mut self.show_entity_bounds, "Entity Bounds");
        });
    }

    fn draw_asset_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Overworld Assets");
        ui.small(format!("{} PNG assets", self.registry.assets().len()));
        ui.separator();

        for category in self.registry.categories() {
            egui::CollapsingHeader::new(category_label(category))
                .default_open(category == "tiles" || category == "props")
                .show(ui, |ui| {
                    for asset in self.registry.in_category(category) {
                        let selected = self.selected_asset.as_deref() == Some(asset.id.as_str());
                        let response = ui
                            .horizontal(|ui| {
                                if let Some(texture) = self.thumbnails.get(&asset.id) {
                                    ui.image((texture.id(), vec2(36.0, 36.0)));
                                } else {
                                    let (rect, _) =
                                        ui.allocate_exact_size(vec2(36.0, 36.0), Sense::hover());
                                    ui.painter().rect_filled(rect, 2.0, Color32::DARK_GRAY);
                                }

                                ui.selectable_label(selected, &asset.id)
                            })
                            .inner;

                        if response.clicked() {
                            self.selected_asset = Some(asset.id.clone());
                            self.selected_item = None;
                            self.active_layer = asset.default_layer;
                            self.tool = if asset.kind == AssetKind::Tile {
                                ToolKind::PaintTile
                            } else {
                                ToolKind::PlaceObject
                            };
                            self.status = format!("Selected {}", asset.id);
                        }
                    }
                });
        }
    }

    fn draw_canvas(&mut self, ui: &mut egui::Ui, ctx: &EguiContext) {
        let desired_size = ui.available_size_before_wrap();
        let (response, painter) = ui.allocate_painter(desired_size, Sense::click_and_drag());
        let rect = response.rect;

        painter.rect_filled(rect, 0.0, Color32::from_rgb(18, 22, 25));
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
        painter.rect_filled(map_rect, 0.0, Color32::from_rgb(24, 28, 29));

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

        if response.hovered() {
            if let Some(mouse) = response.hover_pos() {
                self.mouse_tile = self.screen_to_tile(rect, mouse);
            }
        } else {
            self.mouse_tile = None;
        }

        self.handle_canvas_selection(&response, rect, ctx);
        self.handle_canvas_placement(&response, rect, ctx);
    }

    fn apply_canvas_input(&mut self, response: &egui::Response, ctx: &EguiContext) {
        let space_down = ctx.input(|input| input.key_down(Key::Space));
        let panning = self.tool == ToolKind::Pan || space_down;

        if panning && response.dragged() {
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

    fn handle_canvas_placement(
        &mut self,
        response: &egui::Response,
        canvas_rect: Rect,
        ctx: &EguiContext,
    ) {
        let space_down = ctx.input(|input| input.key_down(Key::Space));
        if self.tool == ToolKind::Select || self.tool == ToolKind::Pan || space_down {
            return;
        }

        let primary_down = ctx.input(|input| input.pointer.primary_down());
        let continuous_paint = self.tool == ToolKind::Erase
            || (self.tool == ToolKind::PaintTile
                && matches!(self.active_layer, LayerKind::Ground | LayerKind::Collision));
        let should_place = if continuous_paint {
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

        if self.tool == ToolKind::Erase {
            self.document.erase_at(self.active_layer, tile_x, tile_y);
            self.status = format!("Erased {}, {}", tile_x, tile_y);
            return;
        }

        if self.active_layer == LayerKind::Collision {
            self.document.place_collision(tile_x, tile_y);
            self.status = format!("Collision {}, {}", tile_x, tile_y);
            return;
        }

        let Some(asset_id) = self.selected_asset().map(|asset| asset.id.clone()) else {
            self.status = "Select an asset first".to_owned();
            return;
        };
        self.selected_item = None;

        match self.active_layer {
            LayerKind::Ground => self.document.place_tile(&asset_id, tile_x, tile_y),
            LayerKind::Decals => self
                .document
                .place_decal(&asset_id, tile_x as f32, tile_y as f32),
            LayerKind::Objects => {
                self.document
                    .place_object(&asset_id, tile_x as f32, tile_y as f32)
            }
            LayerKind::Entities => {
                self.document
                    .place_entity(&asset_id, tile_x as f32, tile_y as f32)
            }
            LayerKind::Zones => self
                .document
                .place_decal(&asset_id, tile_x as f32, tile_y as f32),
            LayerKind::Collision => unreachable!(),
        }

        self.status = format!("Placed {} at {}, {}", asset_id, tile_x, tile_y);
    }

    fn handle_canvas_selection(
        &mut self,
        response: &egui::Response,
        canvas_rect: Rect,
        ctx: &EguiContext,
    ) {
        let space_down = ctx.input(|input| input.key_down(Key::Space));
        if self.tool != ToolKind::Select || space_down {
            return;
        }

        let Some(pointer_pos) = ctx
            .input(|input| input.pointer.interact_pos())
            .or_else(|| response.hover_pos())
        else {
            return;
        };

        if response.drag_started() {
            self.selected_item = self.hit_test_placed_item(canvas_rect, pointer_pos);
            if let Some(selection) = &self.selected_item {
                self.active_layer = selection.layer;
                self.status = format!("Selected {}", selection.label());
            } else {
                self.status = "No object selected".to_owned();
            }
        }

        if response.dragged() && ctx.input(|input| input.pointer.primary_down()) {
            let Some(selection) = self.selected_item.clone() else {
                return;
            };
            let Some([tile_x, tile_y]) = self.screen_to_tile(canvas_rect, pointer_pos) else {
                return;
            };

            self.move_selected_item(&selection, tile_x as f32, tile_y as f32);
            if selection.layer == LayerKind::Ground {
                self.selected_item = Some(SelectedItem {
                    layer: LayerKind::Ground,
                    id: ground_selection_id(tile_x, tile_y),
                });
            }
            self.status = format!("Moved {} to {}, {}", selection.label(), tile_x, tile_y);
            return;
        }

        if response.clicked() {
            self.selected_item = self.hit_test_placed_item(canvas_rect, pointer_pos);
            if let Some(selection) = &self.selected_item {
                self.active_layer = selection.layer;
                self.status = format!("Selected {}", selection.label());
            } else {
                self.status = "No object selected".to_owned();
            }
        }
    }

    fn hit_test_placed_item(&self, canvas_rect: Rect, pointer_pos: Pos2) -> Option<SelectedItem> {
        for entity in self.document.layers.entities.iter().rev() {
            if self
                .object_screen_rect(canvas_rect, &entity.asset, entity.x, entity.y)
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
                .object_screen_rect(canvas_rect, &object.asset, object.x, object.y)
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
                .object_screen_rect(canvas_rect, &decal.asset, decal.x, decal.y)
                .is_some_and(|rect| rect.contains(pointer_pos))
            {
                return Some(SelectedItem {
                    layer: LayerKind::Decals,
                    id: decal.id.clone(),
                });
            }
        }

        for tile in self.document.layers.ground.iter().rev() {
            if self
                .tile_screen_rect(canvas_rect, tile.x, tile.y)
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

    fn move_selected_item(&mut self, selection: &SelectedItem, x: f32, y: f32) {
        match selection.layer {
            LayerKind::Ground => {
                let Some([old_x, old_y]) = parse_ground_selection_id(&selection.id) else {
                    return;
                };
                let new_x = x as i32;
                let new_y = y as i32;
                let Some(index) = self
                    .document
                    .layers
                    .ground
                    .iter()
                    .position(|tile| tile.x == old_x && tile.y == old_y)
                else {
                    return;
                };
                let asset = self.document.layers.ground[index].asset.clone();
                self.document.layers.ground.remove(index);
                self.document.place_tile(&asset, new_x, new_y);
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
            }
            LayerKind::Zones | LayerKind::Collision => {}
        }
    }

    fn draw_layers(&self, canvas_rect: Rect, painter: &egui::Painter) {
        for tile in &self.document.layers.ground {
            let rect = self.tile_screen_rect(canvas_rect, tile.x, tile.y);
            self.draw_asset_image(painter, &tile.asset, rect);
        }

        for decal in &self.document.layers.decals {
            self.draw_object_like(canvas_rect, painter, &decal.asset, decal.x, decal.y);
        }

        for object in &self.document.layers.objects {
            self.draw_object_like(canvas_rect, painter, &object.asset, object.x, object.y);
        }

        for entity in &self.document.layers.entities {
            self.draw_object_like(canvas_rect, painter, &entity.asset, entity.x, entity.y);
        }
    }

    fn draw_object_like(
        &self,
        canvas_rect: Rect,
        painter: &egui::Painter,
        asset_id: &str,
        x: f32,
        y: f32,
    ) {
        if let Some(rect) = self.object_screen_rect(canvas_rect, asset_id, x, y) {
            self.draw_asset_image(painter, asset_id, rect);
        }
    }

    fn object_screen_rect(
        &self,
        canvas_rect: Rect,
        asset_id: &str,
        x: f32,
        y: f32,
    ) -> Option<Rect> {
        let asset = self.registry.get(asset_id)?;
        let tile_size = self.document.tile_size as f32;
        let anchor = self.world_to_screen(
            canvas_rect,
            vec2((x + 0.5) * tile_size, (y + 1.0) * tile_size),
        );
        let size = vec2(asset.default_size[0], asset.default_size[1]) * self.zoom;

        Some(Rect::from_min_size(
            Pos2::new(anchor.x - size.x * 0.5, anchor.y - size.y),
            size,
        ))
    }

    fn draw_asset_image(&self, painter: &egui::Painter, asset_id: &str, rect: Rect) {
        if let Some(texture) = self.thumbnails.get(asset_id) {
            painter.image(
                texture.id(),
                rect,
                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            painter.rect_filled(rect, 1.0, Color32::from_rgb(80, 80, 90));
        }
    }

    fn tile_screen_rect(&self, canvas_rect: Rect, x: i32, y: i32) -> Rect {
        let tile_size = self.document.tile_size as f32;
        let world = vec2(x as f32 * tile_size, y as f32 * tile_size);
        Rect::from_min_size(
            self.world_to_screen(canvas_rect, world),
            vec2(tile_size, tile_size) * self.zoom,
        )
    }

    fn draw_collision(&self, canvas_rect: Rect, painter: &egui::Painter) {
        let tile_size = self.document.tile_size as f32;
        for cell in &self.document.layers.collision {
            if !cell.solid {
                continue;
            }

            let world = vec2(cell.x as f32 * tile_size, cell.y as f32 * tile_size);
            let rect = Rect::from_min_size(
                self.world_to_screen(canvas_rect, world),
                vec2(tile_size, tile_size) * self.zoom,
            );
            painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(255, 48, 48, 80));
        }
    }

    fn draw_entity_bounds(&self, canvas_rect: Rect, painter: &egui::Painter) {
        let tile_size = self.document.tile_size as f32;
        for entity in &self.document.layers.entities {
            let world = vec2(entity.x * tile_size, entity.y * tile_size);
            let rect = Rect::from_min_size(
                self.world_to_screen(canvas_rect, world),
                vec2(tile_size, tile_size) * self.zoom,
            );
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(1.5, Color32::from_rgb(120, 210, 255)),
            );
        }
    }

    fn draw_selection_bounds(&self, canvas_rect: Rect, painter: &egui::Painter) {
        let Some(selection) = &self.selected_item else {
            return;
        };

        let rect = match selection.layer {
            LayerKind::Ground => parse_ground_selection_id(&selection.id).and_then(|[x, y]| {
                self.document
                    .layers
                    .ground
                    .iter()
                    .any(|tile| tile.x == x && tile.y == y)
                    .then(|| self.tile_screen_rect(canvas_rect, x, y))
            }),
            LayerKind::Decals => self
                .document
                .layers
                .decals
                .iter()
                .find(|instance| instance.id == selection.id)
                .and_then(|instance| {
                    self.object_screen_rect(canvas_rect, &instance.asset, instance.x, instance.y)
                }),
            LayerKind::Objects => self
                .document
                .layers
                .objects
                .iter()
                .find(|instance| instance.id == selection.id)
                .and_then(|instance| {
                    self.object_screen_rect(canvas_rect, &instance.asset, instance.x, instance.y)
                }),
            LayerKind::Entities => self
                .document
                .layers
                .entities
                .iter()
                .find(|instance| instance.id == selection.id)
                .and_then(|instance| {
                    self.object_screen_rect(canvas_rect, &instance.asset, instance.x, instance.y)
                }),
            LayerKind::Zones | LayerKind::Collision => None,
        };

        if let Some(rect) = rect {
            painter.rect_stroke(rect.expand(3.0), 2.0, Stroke::new(2.0, Color32::YELLOW));
        }
    }

    fn draw_status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mouse = self
                .mouse_tile
                .map(|tile| format!("{}, {}", tile[0], tile[1]))
                .unwrap_or_else(|| "-".to_owned());
            let asset = self.selected_asset.as_deref().unwrap_or("none");
            let selected_item = self
                .selected_item
                .as_ref()
                .map(SelectedItem::label)
                .unwrap_or_else(|| "none".to_owned());

            ui.label(format!("Mouse Tile: {mouse}"));
            ui.separator();
            ui.label(format!("Selected: {asset}"));
            ui.separator();
            ui.label(format!("Selection: {selected_item}"));
            ui.separator();
            ui.label(format!("Layer: {}", self.active_layer.label()));
            ui.separator();
            ui.label(format!("Zoom: {:.0}%", self.zoom * 100.0));
            ui.separator();
            ui.label(&self.status);
        });
    }

    fn world_to_screen(&self, canvas_rect: Rect, world: Vec2) -> Pos2 {
        canvas_rect.min + self.pan + world * self.zoom
    }

    fn screen_to_tile(&self, canvas_rect: Rect, screen: Pos2) -> Option<[i32; 2]> {
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
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &EguiContext, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| self.draw_top_bar(ui));
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| self.draw_status_bar(ui));
        egui::SidePanel::left("asset_panel")
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.draw_asset_panel(ui));
            });
        egui::CentralPanel::default().show(ctx, |ui| self.draw_canvas(ui, ctx));
    }
}

fn draw_grid(
    painter: &egui::Painter,
    clip_rect: Rect,
    map_rect: Rect,
    width: u32,
    height: u32,
    tile_size: f32,
) {
    if tile_size < 4.0 {
        return;
    }

    let stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(210, 220, 220, 34));
    let clipped = map_rect.intersect(clip_rect);
    if clipped.is_negative() {
        return;
    }

    for x in 0..=width {
        let screen_x = map_rect.min.x + x as f32 * tile_size;
        if screen_x < clip_rect.left() || screen_x > clip_rect.right() {
            continue;
        }
        painter.line_segment(
            [
                Pos2::new(screen_x, clipped.top()),
                Pos2::new(screen_x, clipped.bottom()),
            ],
            stroke,
        );
    }

    for y in 0..=height {
        let screen_y = map_rect.min.y + y as f32 * tile_size;
        if screen_y < clip_rect.top() || screen_y > clip_rect.bottom() {
            continue;
        }
        painter.line_segment(
            [
                Pos2::new(clipped.left(), screen_y),
                Pos2::new(clipped.right(), screen_y),
            ],
            stroke,
        );
    }
}

fn load_thumbnail(ctx: &EguiContext, asset: &AssetEntry) -> Result<TextureHandle> {
    let image = image::ImageReader::open(&asset.path)
        .with_context(|| format!("failed to open {}", asset.path.display()))?
        .decode()
        .with_context(|| format!("failed to decode {}", asset.path.display()))?;
    let thumbnail = image.thumbnail(128, 128).to_rgba8();
    let (width, height) = thumbnail.dimensions();
    let pixels = thumbnail.into_raw();
    let color_image =
        egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &pixels);

    Ok(ctx.load_texture(&asset.id, color_image, TextureOptions::LINEAR))
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn display_project_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn category_label(category: &str) -> &str {
    match category {
        "tiles" => "Tiles",
        "decals" => "Decals",
        "props" => "Props",
        "flora" => "Flora",
        "fauna" => "Fauna",
        "structures" => "Structures",
        "ruins" => "Ruins",
        "interactables" => "Interactables",
        "pickups" => "Pickups",
        "zones" => "Zones",
        _ => category,
    }
}

fn ground_selection_id(x: i32, y: i32) -> String {
    format!("{x},{y}")
}

fn parse_ground_selection_id(id: &str) -> Option<[i32; 2]> {
    let (x, y) = id.split_once(',')?;
    Some([x.parse().ok()?, y.parse().ok()?])
}
