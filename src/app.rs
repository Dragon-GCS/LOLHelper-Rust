use std::sync::{Arc, atomic::Ordering};

use eframe::App;
use eframe::egui::{
    self, Align, Checkbox, Color32, CursorIcon, DragValue, FontData, FontDefinitions, Frame, Grid,
    Id, Label, Layout, Modal, ScrollArea, Separator, Vec2, Widget,
};
use log::error;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::{
    context::{Champion, HelperContext},
    lcu::{LcuClient, start_event_listener},
    log::LOGS,
};

// 应用常量
const APP_NAME: &str = "Champion Selector";
const FRAME_MARGIN: f32 = 5.0;
const SEPARATOR_SPACING: f32 = 10.0;
const BUTTON_SPACING: f32 = 30.0;
const CHAMPION_FRAME_SIZE: Vec2 = Vec2::new(200.0, 200.0);

/// 主应用程序状态
pub struct MyApp {
    ctx: Arc<HelperContext>,
    client: Arc<RwLock<LcuClient>>,
    rt: tokio::runtime::Runtime,
    cancel_token: Arc<CancellationToken>,
    // 英雄选择窗口是否打开
    champion_pick_window_open: bool,
    // modal是否打开
    modal_open: bool,
}

/// 在网格中添加标签和控件的宏
macro_rules! add_grid_row {
    ($ui:expr, $($widget:expr),+) => {
        $($ui.add($widget);)+
        $ui.end_row();
    };
}

/// 安全地在Vec之间移动元素的宏
macro_rules! move_item_safe {
    // 移动到指定位置
    ($src:expr, $dst:expr, $from:expr, $to:expr) => {
        if $from < $src.len() {
            let item = $src.remove($from);
            let insert_pos = $to.min($dst.len());
            $dst.insert(insert_pos, item);
        }
    };
    // 移动到开头
    ($src:expr, $dst:expr, $from:expr) => {
        if $from < $src.len() {
            let item = $src.remove($from);
            $dst.insert(0, item);
        }
    };
}

/// 渲染英雄列表的通用宏
macro_rules! render_champion_list {
    ($ui:expr,  $header:expr, $content:expr) => {
        $ui.vertical_centered(|ui| ui.heading($header));
        Frame::default()
            .inner_margin(FRAME_MARGIN)
            .fill(Color32::LIGHT_GRAY)
            .show($ui, |ui| {
                ui.set_min_size(CHAMPION_FRAME_SIZE);
                ui.set_max_height(CHAMPION_FRAME_SIZE.y);
                ScrollArea::vertical().id_salt($header).show(ui, $content);
            });
    };
}

impl App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("Settings")
            .resizable(false)
            .show(ctx, |ui| self.setting_panel(ui));

        egui::CentralPanel::default().show(ctx, |ui| self.log_panel(ctx, ui));

        if self.champion_pick_window_open {
            egui::Window::new("select")
                .collapsible(false)
                .title_bar(false)
                .resizable([false, true])
                .show(ctx, |ui| self.champion_pick_window(ui));
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // 保存设置到存储
        storage.set_string(
            "auto_pick",
            serde_json::to_string(&*self.ctx.auto_pick.read().unwrap()).unwrap(),
        );
        storage.set_string(
            "auto_accepted_delay",
            serde_json::to_string(&*self.ctx.auto_accepted_delay.read().unwrap()).unwrap(),
        );
        storage.set_string(
            "auto_send_analysis",
            serde_json::to_string(&self.ctx.auto_send_analysis.load(Ordering::Relaxed)).unwrap(),
        );
    }
}

#[derive(Default)]
struct ChampionPickState {
    drag_from: Option<usize>,
    drag_to: Option<usize>,
    select_index: Option<usize>,
    unselect_index: Option<usize>,
}

impl MyApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "msyh".to_owned(),
            Arc::new(FontData::from_static(include_bytes!(
                "../MapleMono-NF-CN-Regular.ttf"
            ))),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "msyh".to_owned());
        fonts
            .families
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap()
            .insert(0, "msyh".to_owned());

        cc.egui_ctx.set_fonts(fonts);
        cc.egui_ctx.set_zoom_factor(1.5);

        let ctx = if let Some(storage) = cc.storage {
            HelperContext::from_storage(storage)
        } else {
            HelperContext::default()
        };

        Self {
            ctx: Arc::new(ctx),
            rt: tokio::runtime::Runtime::new().unwrap(),
            client: Arc::new(RwLock::new(LcuClient::default())),
            cancel_token: Arc::new(CancellationToken::new()),
            champion_pick_window_open: false,
            modal_open: false,
        }
    }

    fn setting_panel(&mut self, ui: &mut egui::Ui) {
        self.render_settings_controls(ui);
        self.render_user_info(ui);
        self.render_control_buttons(ui);
    }

    /// 渲染设置控件区域
    fn render_settings_controls(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
            ui.add_space(FRAME_MARGIN);
            Grid::new("settings_grid")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .show(ui, |ui| {
                    // 英雄选择按钮
                    if ui.button("自动选择英雄").clicked() {
                        self.champion_pick_window_open = true;
                    }
                    ui.add(Checkbox::without_text(
                        &mut self.ctx.auto_pick.write().unwrap().enabled,
                    ));
                    ui.end_row();

                    // 自动接受延迟
                    add_grid_row!(
                        ui,
                        Label::new("自动接受延迟"),
                        DragValue::new(&mut *self.ctx.auto_accepted_delay.write().unwrap())
                            .range(0.0..=100.0)
                            .suffix(" s")
                    );

                    // 自动分析
                    add_grid_row!(
                        ui,
                        Label::new("自动发送消息"),
                        Checkbox::without_text(
                            &mut self.ctx.auto_send_analysis.load(Ordering::Relaxed)
                        )
                    );
                });
        });
    }

    /// 渲染用户信息区域
    fn render_user_info(&self, ui: &mut egui::Ui) {
        Separator::default().spacing(SEPARATOR_SPACING).ui(ui);

        ui.vertical(|ui| {
            let me = self.ctx.me.read().unwrap();
            ui.label(format!("名称: {}", me.game_name));
            ui.add_space(FRAME_MARGIN);
            ui.label(format!("等级: {}", me.summoner_level));
            ui.add_space(FRAME_MARGIN);
            ui.label(format!("UUID: {}", me.puuid));
        });
    }

    /// 渲染控制按钮区域
    fn render_control_buttons(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(
            Layout::bottom_up(Align::Center).with_cross_justify(true),
            |ui| {
                ui.add_space(BUTTON_SPACING);

                // 开始/停止按钮
                let button_text = if self.ctx.listening.load(Ordering::Relaxed) {
                    "停止助手"
                } else {
                    "启动助手"
                };
                if ui.button(button_text).clicked() {
                    if !self.ctx.listening.load(Ordering::Relaxed) {
                        let client = self.client.clone();
                        let ctx = self.ctx.clone();
                        let cancel_token = self.cancel_token.clone();
                        {
                            self.ctx.listening.store(true, Ordering::Relaxed);
                        }
                        self.rt.spawn(async move {
                            start_event_listener(client, ctx.clone(), cancel_token)
                                .await
                                .unwrap_or_else(|e| {
                                    error!("启动事件监听失败: {e}");
                                    ctx.listening.store(false, Ordering::Relaxed);
                                })
                        });
                    } else {
                        self.cancel_token.cancel();
                        self.cancel_token = Arc::new(CancellationToken::new());
                        self.ctx.reset();
                    }
                }
            },
        );
    }

    fn log_panel(&self, ctx: &egui::Context, ui: &mut egui::Ui) {
        Frame::new()
            .inner_margin(FRAME_MARGIN)
            .corner_radius(5.0)
            .fill(ctx.style().visuals.code_bg_color)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for log in LOGS.read().unwrap().iter() {
                            ui.code(format!("{}", log));
                        }
                    });
            });
    }

    fn champion_pick_window(&mut self, ui: &mut egui::Ui) {
        let mut state = ChampionPickState::default();

        ui.columns(2, |uis| {
            let (left, right) = uis.split_at_mut(1);
            let (left, right) = (&mut left[0], &mut right[0]);

            // 左侧：未选中英雄列表
            render_champion_list!(left, "可用英雄", |ui| {
                for (idx, name) in self
                    .ctx
                    .auto_pick
                    .read()
                    .unwrap()
                    .unselected
                    .iter()
                    .enumerate()
                {
                    if ui
                        .label(name.1.clone())
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .clicked()
                    {
                        state.select_index = Some(idx);
                    }
                }
            });
            // 右侧：已选中英雄列表（支持拖拽）
            render_champion_list!(right, "已选英雄", |ui| {
                for (idx, name) in self
                    .ctx
                    .auto_pick
                    .read()
                    .unwrap()
                    .selected
                    .iter()
                    .enumerate()
                {
                    self.render_draggable_champion_item(ui, idx, name, &mut state);
                }
            });
        });

        // 处理所有操作
        self.handle_champion_operations(state);

        // 控制按钮
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("清空已选").clicked() {
                let mut auto_pick = self.ctx.auto_pick.write().unwrap();
                let selected_clone = auto_pick.selected.clone();
                auto_pick.unselected.extend(selected_clone);
                auto_pick
                    .unselected
                    .sort_by_key(|champion| champion.1.clone());
                auto_pick.selected.clear();
            }
            if ui.button("更新可用英雄").clicked() {
                if !self.ctx.listening.load(Ordering::Relaxed) {
                    self.modal_open = true;
                } else {
                    let client = self.client.clone();
                    let ctx = self.ctx.clone();
                    self.rt.spawn(async move {
                        let champions = client
                            .read()
                            .await
                            .get_owned_champions()
                            .await
                            .unwrap_or_else(|e| {
                                error!("获取英雄列表失败: {e}");
                                vec![]
                            });
                        let selected = {
                            ctx.auto_pick
                                .read()
                                .unwrap()
                                .selected
                                .iter()
                                .map(|champ| champ.0)
                                .collect::<Vec<u16>>()
                        };
                        let mut auto_pick = ctx.auto_pick.write().unwrap();
                        auto_pick.unselected = champions
                            .into_iter()
                            .filter(|champ| !selected.contains(&champ.0))
                            .collect();
                    });
                }
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("关闭").clicked() {
                    self.champion_pick_window_open = false;
                }
            });
        });

        if self.modal_open && self.render_modal(ui, "请先启动游戏助手") {
            self.modal_open = false;
        }
    }

    fn render_modal(&self, ui: &mut egui::Ui, message: &str) -> bool {
        let modal = Modal::new(Id::new("message")).show(ui.ctx(), |ui| {
            ui.label(message);
        });
        modal.should_close()
    }

    fn render_draggable_champion_item(
        &self,
        ui: &mut egui::Ui,
        idx: usize,
        name: &Champion,
        state: &mut ChampionPickState,
    ) {
        let drag_id = egui::Id::new(("selected_champion", idx));
        let response = ui
            .dnd_drag_source(drag_id, idx, |ui| ui.label(name.1.clone()))
            .response
            .interact(egui::Sense::click_and_drag());
        if response.clicked() && state.unselect_index.is_none() {
            state.unselect_index = Some(idx);
        }

        // 处理拖拽悬停和释放
        if let (Some(pointer_pos), Some(hovered_payload)) = (
            ui.input(|i| i.pointer.interact_pos()),
            response.dnd_hover_payload::<usize>(),
        ) {
            let insert_pos =
                self.calculate_insert_position(ui, &response, pointer_pos, *hovered_payload, idx);

            if let Some(dragged_payload) = response.dnd_release_payload::<usize>() {
                state.drag_from = Some(*dragged_payload);
                state.drag_to = Some(insert_pos);
            }
        }
    }

    fn calculate_insert_position(
        &self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        pointer_pos: egui::Pos2,
        hovered_payload: usize,
        current_idx: usize,
    ) -> usize {
        let rect = response.rect;
        let stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);

        if hovered_payload == current_idx {
            // 拖拽到自己身上
            ui.painter().hline(rect.x_range(), rect.center().y, stroke);
            current_idx
        } else if pointer_pos.y < rect.center().y {
            // 拖拽到上方
            ui.painter().hline(rect.x_range(), rect.top(), stroke);
            current_idx
        } else {
            // 拖拽到下方
            ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
            current_idx + 1
        }
    }

    fn handle_champion_operations(&mut self, state: ChampionPickState) {
        let mut auto_pick = self.ctx.auto_pick.write().unwrap();
        match (
            state.select_index,
            state.drag_from,
            state.drag_to,
            state.unselect_index,
        ) {
            // 从未选中列表选择英雄
            (Some(from), None, None, None) => {
                move_item_safe!(auto_pick.unselected, auto_pick.selected, from);
            }
            // 拖拽重排序
            (None, Some(from), Some(to), None) if from != to => {
                move_item_safe!(auto_pick.selected, auto_pick.selected, from, to);
            }
            // 取消选择英雄
            (None, None, None, Some(from)) => {
                move_item_safe!(auto_pick.selected, auto_pick.unselected, from);
            }
            // 其他情况不处理（避免冲突）
            _ => {}
        }
    }
}
