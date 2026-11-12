use gpui::{div, prelude::*, px, ElementId, FontWeight, Render, SharedString, Window};
use homer_core::{visible_segments, Character, Message, SegmentType};
use std::time::Duration;

use crate::app::{HomerApp, SidebarTab};
use crate::{theme, ui};

impl Render for HomerApp {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        self.start_event_loop(window, cx);
        if self.first_render {
            self.first_render = false;
            self.scroll.scroll_to_bottom();
        }
        let mut root = div()
            .size_full()
            .flex()
            .flex_row()
            .relative()
            .bg(theme::bg())
            .text_color(theme::text())
            .font_family(theme::ui_font())
            .text_size(px(13.))
            .on_action(cx.listener(Self::submit_message))
            .on_action(cx.listener(Self::new_conversation_action))
            .on_action(cx.listener(Self::open_settings_action))
            .on_action(cx.listener(Self::close_modal_action))
            .child(self.render_sidebar(window, cx))
            .child(self.render_chat(window, cx));
        if self.panel_open {
            root = root.child(self.render_panel(window, cx));
        }
        if self.modal != crate::app::Modal::None {
            root = root.child(self.render_modal(window, cx));
        }
        if let Some((message, _)) = &self.toast {
            root = root.child(
                div()
                    .absolute()
                    .bottom(px(96.))
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .px(px(14.))
                            .py(px(8.))
                            .rounded(px(7.))
                            .bg(theme::text())
                            .text_color(theme::bg())
                            .text_size(px(12.))
                            .shadow_lg()
                            .child(message.clone()),
                    ),
            );
        }
        root
    }
}

impl HomerApp {
    fn start_event_loop(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        if self.event_loop_started {
            return;
        }
        self.event_loop_started = true;
        let rx = self.rx.clone();
        let entity = cx.entity();
        window
            .spawn(cx, async move |cx| {
                while let Ok(first) = rx.recv().await {
                    let mut events = vec![first];
                    gpui::Timer::after(Duration::from_millis(16)).await;
                    while let Ok(event) = rx.try_recv() {
                        events.push(event);
                    }
                    let result = entity.update_in(cx, |app, window, cx| {
                        app.handle_events(events, window, cx);
                    });
                    if result.is_err() {
                        break;
                    }
                }
            })
            .detach();
    }

    fn render_sidebar(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> gpui::Div {
        let brand = div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(52.))
            .pl(px(18.))
            .pr(px(12.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme::text())
                    .child("Homer"),
            )
            .child(
                div()
                    .id("sidebar-new")
                    .size(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .text_color(theme::muted())
                    .text_size(px(19.))
                    .cursor_pointer()
                    .hover(|s| s.bg(gpui::rgb(0x302c2710)).text_color(theme::text()))
                    .child("＋")
                    .on_click(cx.listener(
                        |this, _: &gpui::ClickEvent, window, cx| match this.tab {
                            SidebarTab::Conversations => this.open_new_conversation(cx),
                            SidebarTab::Characters => this.open_character(None, window, cx),
                        },
                    )),
            );

        let tab_conversations = self.tab == SidebarTab::Conversations;
        let tabs = div()
            .flex()
            .gap(px(16.))
            .px(px(18.))
            .border_b_1()
            .border_color(theme::line())
            .child(
                div()
                    .id("tab-conversations")
                    .pb(px(8.))
                    .text_size(px(11.5))
                    .text_color(if tab_conversations {
                        theme::text()
                    } else {
                        theme::faint()
                    })
                    .border_b_1()
                    .border_color(if tab_conversations {
                        theme::text()
                    } else {
                        theme::line()
                    })
                    .cursor_pointer()
                    .child("会話")
                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                        this.tab = SidebarTab::Conversations;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("tab-characters")
                    .pb(px(8.))
                    .text_size(px(11.5))
                    .text_color(if tab_conversations {
                        theme::faint()
                    } else {
                        theme::text()
                    })
                    .border_b_1()
                    .border_color(if tab_conversations {
                        theme::line()
                    } else {
                        theme::text()
                    })
                    .cursor_pointer()
                    .child("キャラクター")
                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                        this.tab = SidebarTab::Characters;
                        cx.notify();
                    })),
            );

        let mut rows: Vec<gpui::AnyElement> = Vec::new();
        if self.tab == SidebarTab::Conversations {
            if self.conversations.is_empty() {
                rows.push(
                    empty_list(
                        "まだ物語がありません。",
                        "新しい会話を始める",
                        cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                            this.open_new_conversation(cx)
                        }),
                    )
                    .into_any_element(),
                );
            }
            let items: Vec<(homer_core::Conversation, Option<Character>)> = self
                .conversations
                .iter()
                .map(|conversation| {
                    let character = conversation
                        .character_id
                        .as_deref()
                        .and_then(|id| self.characters.iter().find(|c| c.id == id))
                        .cloned();
                    (conversation.clone(), character)
                })
                .collect();
            for (conversation, character) in items {
                let active = self.active_id.as_deref() == Some(conversation.id.as_str());
                let subtitle = if !conversation.scene.trim().is_empty() {
                    conversation.scene.clone()
                } else if !conversation.scenario.trim().is_empty() {
                    conversation.scenario.clone()
                } else {
                    character
                        .as_ref()
                        .map(|c| c.description.clone())
                        .unwrap_or_else(|| "物語".into())
                };
                let subtitle = subtitle.replace('\n', " ");
                let id = conversation.id.clone();
                let row = div()
                    .id(ElementId::Name(
                        format!("conversation-{}", conversation.id).into(),
                    ))
                    .flex()
                    .items_center()
                    .gap(px(9.))
                    .px(px(9.))
                    .py(px(7.))
                    .min_h(px(49.))
                    .rounded(px(7.))
                    .cursor_pointer()
                    .when(active, |row| row.bg(gpui::rgb(0xe3dccf)))
                    .when(!active, |row| row.hover(|s| s.bg(gpui::rgb(0x3c342b0b))))
                    .child(match &character {
                        Some(character) => ui::avatar(&character.name, &character.accent, 30.),
                        None => ui::narrator_avatar(30.),
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .min_w(px(0.))
                            .flex_1()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .flex_1()
                                            .overflow_hidden()
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme::text())
                                            .truncate()
                                            .child(conversation.title.clone()),
                                    )
                                    .child(
                                        div()
                                            .flex_none()
                                            .text_size(px(9.5))
                                            .text_color(theme::faint())
                                            .child(theme::format_time(conversation.updated_at)),
                                    ),
                            )
                            .child(
                                div()
                                    .overflow_hidden()
                                    .text_size(px(11.))
                                    .text_color(theme::muted())
                                    .truncate()
                                    .child(subtitle),
                            ),
                    )
                    .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                        this.select_conversation(id.clone(), cx);
                    }));
                rows.push(row.into_any_element());
            }
        } else {
            if self.characters.is_empty() {
                rows.push(
                    empty_list(
                        "キャラクターはまだいません。",
                        "キャラクターを作る",
                        cx.listener(|this, _: &gpui::ClickEvent, window, cx| {
                            this.open_character(None, window, cx)
                        }),
                    )
                    .into_any_element(),
                );
            }
            for character in self.characters.clone() {
                let id = character.id.clone();
                let description = if character.description.trim().is_empty() {
                    "設定なし".to_string()
                } else {
                    character.description.replace('\n', " ")
                };
                rows.push(
                    div()
                        .id(ElementId::Name(
                            format!("character-{}", character.id).into(),
                        ))
                        .flex()
                        .items_center()
                        .gap(px(9.))
                        .px(px(9.))
                        .py(px(7.))
                        .min_h(px(49.))
                        .rounded(px(7.))
                        .cursor_pointer()
                        .hover(|s| s.bg(gpui::rgb(0x3c342b0b)))
                        .child(ui::avatar(&character.name, &character.accent, 30.))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .min_w(px(0.))
                                .flex_1()
                                .child(
                                    div()
                                        .text_size(px(12.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(character.name.clone()),
                                )
                                .child(
                                    div()
                                        .overflow_hidden()
                                        .text_size(px(11.))
                                        .text_color(theme::muted())
                                        .truncate()
                                        .child(description),
                                ),
                        )
                        .on_click(cx.listener(move |this, _: &gpui::ClickEvent, window, cx| {
                            this.open_character(Some(id.clone()), window, cx);
                        }))
                        .into_any_element(),
                );
            }
        }

        let footer = div()
            .flex()
            .items_center()
            .h(px(48.))
            .px(px(8.))
            .border_t_1()
            .border_color(theme::line())
            .child(
                div()
                    .id("open-settings")
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .h(px(30.))
                    .px(px(9.))
                    .rounded(px(6.))
                    .text_color(theme::muted())
                    .text_size(px(12.))
                    .cursor_pointer()
                    .hover(|s| s.bg(gpui::rgb(0x3c342b0d)).text_color(theme::text()))
                    .child("⚙")
                    .child("設定")
                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, window, cx| {
                        this.open_settings(window, cx)
                    })),
            );

        div()
            .w(px(272.))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .bg(theme::sidebar())
            .border_r_1()
            .border_color(theme::line())
            .child(brand)
            .child(tabs)
            .child(
                div()
                    .id("sidebar-scroll")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .track_scroll(&self.sidebar_scroll)
                    .p(px(8.))
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .children(rows),
            )
            .child(footer)
    }

    fn render_chat(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> gpui::Div {
        let Some(conversation) = self.active_conversation().cloned() else {
            return div()
                .flex_1()
                .min_w(px(0.))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme::bg())
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(8.))
                        .max_w(px(380.))
                        .child(ui::narrator_avatar(52.))
                        .child(
                            div()
                                .text_size(px(16.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .mt(px(4.))
                                .child("物語は、まだ始まっていません。"),
                        )
                        .child(
                            div()
                                .text_size(px(12.5))
                                .text_color(theme::muted())
                                .text_center()
                                .child("キャラクターと場面を決めて、会話を始めましょう。"),
                        )
                        .child(
                            div()
                                .flex()
                                .gap(px(8.))
                                .mt(px(14.))
                                .child(ui::button("新しい会話", true).id("welcome-new").on_click(
                                    cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                        this.open_new_conversation(cx)
                                    }),
                                ))
                                .child(
                                    ui::button("キャラクターを作る", false)
                                        .id("welcome-character")
                                        .on_click(cx.listener(
                                            |this, _: &gpui::ClickEvent, window, cx| {
                                                this.open_character(None, window, cx)
                                            },
                                        )),
                                ),
                        ),
                );
        };

        let character = self.active_character().cloned();
        let is_messenger = conversation.is_messenger();
        let title = conversation.title.clone();
        let subtitle = if !conversation.scenario.trim().is_empty() {
            conversation.scenario.clone()
        } else if let Some(character) = &character {
            character.description.clone()
        } else if !conversation.scene.trim().is_empty() {
            conversation.scene.clone()
        } else {
            "自由な物語".to_string()
        };
        let subtitle = subtitle.replace('\n', " ");

        let header = div()
            .h(px(60.))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(16.))
            .px(px(20.))
            .border_b_1()
            .border_color(theme::line())
            .child(
                div()
                    .id("chat-identity")
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .min_w(px(0.))
                    .rounded(px(7.))
                    .cursor_pointer()
                    .child(match &character {
                        Some(character) => ui::avatar(&character.name, &character.accent, 30.),
                        None => ui::narrator_avatar(30.),
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .min_w(px(0.))
                            .child(
                                div()
                                    .text_size(px(13.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .overflow_hidden()
                                    .truncate()
                                    .child(title),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme::muted())
                                    .overflow_hidden()
                                    .max_w(px(370.))
                                    .truncate()
                                    .child(subtitle),
                            ),
                    )
                    .on_click(
                        cx.listener(|this, _: &gpui::ClickEvent, _, cx| this.toggle_panel(cx)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .flex_none()
                    .child(
                        div()
                            .flex()
                            .p(px(2.))
                            .gap(px(2.))
                            .rounded(px(7.))
                            .bg(theme::surface_2())
                            .border_1()
                            .border_color(theme::line())
                            .child(mode_button("物語", !is_messenger, "mode-story", cx))
                            .child(mode_button(
                                "メッセンジャー",
                                is_messenger,
                                "mode-messenger",
                                cx,
                            )),
                    )
                    .child(ui::icon_button("▤", 28.).id("toggle-panel").on_click(
                        cx.listener(|this, _: &gpui::ClickEvent, _, cx| this.toggle_panel(cx)),
                    )),
            );

        let mut thread_children: Vec<gpui::AnyElement> = Vec::new();
        if self.loading_messages {
            thread_children.push(
                div()
                    .flex()
                    .justify_center()
                    .py(px(24.))
                    .text_size(px(12.))
                    .text_color(theme::faint())
                    .child("読み込み中…")
                    .into_any_element(),
            );
        } else if self.messages.is_empty() {
            thread_children.push(self.render_intro(&conversation, character.as_ref(), cx));
        } else {
            let mode = conversation.mode.clone();
            let show_metrics = self.settings.show_metrics;
            for message in &self.messages {
                thread_children.push(
                    render_message(message, character.as_ref(), &mode, show_metrics)
                        .into_any_element(),
                );
            }
        }

        let streaming_here = self
            .streaming
            .as_ref()
            .map(|s| s.conversation_id == conversation.id)
            .unwrap_or(false);
        if streaming_here {
            let text = self
                .streaming
                .as_ref()
                .map(|s| s.text.clone())
                .unwrap_or_default();
            thread_children.push(
                render_streaming(&conversation.mode, character.as_ref(), &text).into_any_element(),
            );
        }

        let scroller = div()
            .id("message-scroll")
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .on_scroll_wheel(cx.listener(|this, event: &gpui::ScrollWheelEvent, _, _cx| {
                let delta = event.delta.pixel_delta(px(20.)).y;
                if delta > px(0.) {
                    this.auto_follow = false;
                }
            }))
            .child(
                div()
                    .w_full()
                    .max_w(px(768.))
                    .mx_auto()
                    .px(px(30.))
                    .py(px(24.))
                    .flex()
                    .flex_col()
                    .children(thread_children),
            );

        div()
            .flex_1()
            .min_w(px(0.))
            .h_full()
            .flex()
            .flex_col()
            .bg(theme::bg())
            .child(header)
            .child(scroller)
            .child(self.render_composer(streaming_here, cx))
    }

    fn render_intro(
        &self,
        conversation: &homer_core::Conversation,
        character: Option<&Character>,
        cx: &mut gpui::Context<Self>,
    ) -> gpui::AnyElement {
        let character_id = character.map(|c| c.id.clone());
        let mut intro = div()
            .flex()
            .flex_col()
            .items_center()
            .text_center()
            .gap(px(9.))
            .py(px(70.))
            .max_w(px(480.))
            .mx_auto()
            .child(match character {
                Some(character) => ui::avatar(&character.name, &character.accent, 52.),
                None => ui::narrator_avatar(52.),
            })
            .child(
                div()
                    .text_size(px(18.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .mt(px(6.))
                    .child(conversation.title.clone()),
            );
        if !conversation.scenario.trim().is_empty() {
            intro = intro.child(
                div()
                    .font_family(theme::serif_font())
                    .text_size(px(13.))
                    .line_height(px(24.))
                    .text_color(gpui::rgb(0x5f574c))
                    .child(conversation.scenario.clone()),
            );
        }
        if let Some(character) = character {
            if !character.description.trim().is_empty() {
                intro = intro.child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(21.))
                        .text_color(theme::muted())
                        .child(character.description.clone()),
                );
            }
        }
        intro = intro
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme::faint())
                    .mt(px(14.))
                    .child("最初の言葉を書いてください。"),
            )
            .child(
                div()
                    .id("intro-edit-character")
                    .text_size(px(11.5))
                    .text_color(theme::muted())
                    .cursor_pointer()
                    .hover(|s| s.text_color(theme::text()))
                    .child(if character.is_some() {
                        "キャラクターを編集"
                    } else {
                        "キャラクターを作る"
                    })
                    .on_click(cx.listener(move |this, _: &gpui::ClickEvent, window, cx| {
                        this.open_character(character_id.clone(), window, cx)
                    })),
            );
        intro.into_any_element()
    }

    fn render_composer(&self, streaming_here: bool, cx: &mut gpui::Context<Self>) -> gpui::Div {
        let mut area = div().flex_none().px(px(16.)).pb(px(6.)).flex().flex_col();

        if let Some(error) = &self.error {
            let retryable = error.retryable;
            area =
                area.child(
                    div()
                        .max_w(px(768.))
                        .mx_auto()
                        .w_full()
                        .mb(px(8.))
                        .px(px(12.))
                        .py(px(9.))
                        .flex()
                        .items_start()
                        .gap(px(8.))
                        .rounded(px(8.))
                        .bg(gpui::rgb(theme::ERROR_BG))
                        .border_1()
                        .border_color(gpui::rgb(theme::ERROR_LINE))
                        .text_color(gpui::rgb(0x6d3f33))
                        .child(div().flex_none().mt(px(1.)).text_size(px(13.)).child("!"))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(20.))
                                        .child(error.message.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(9.5))
                                        .text_color(gpui::rgb(0xa3796c))
                                        .child(kind_label(&error.kind)),
                                ),
                        )
                        .when(retryable, |row| {
                            row.child(
                                ui::button("再試行", false)
                                    .id("retry-generation")
                                    .h(px(26.))
                                    .text_size(px(11.5))
                                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                        this.retry_generation(cx)
                                    })),
                            )
                        })
                        .child(ui::icon_button("×", 24.).id("dismiss-error").on_click(
                            cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.error = None;
                                cx.notify();
                            }),
                        )),
                );
        }

        if streaming_here {
            area = area.child(
                div()
                    .max_w(px(768.))
                    .mx_auto()
                    .w_full()
                    .pb(px(6.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .text_size(px(11.))
                            .text_color(theme::muted())
                            .child(div().size(px(5.)).rounded_full().bg(theme::accent()))
                            .child("応答を生成中"),
                    )
                    .child(
                        div()
                            .id("stop-generation")
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .h(px(23.))
                            .px(px(8.))
                            .rounded(px(5.))
                            .bg(theme::surface())
                            .border_1()
                            .border_color(theme::line())
                            .text_size(px(11.))
                            .text_color(theme::muted())
                            .cursor_pointer()
                            .hover(|s| {
                                s.text_color(theme::text())
                                    .border_color(theme::line_strong())
                            })
                            .child("■")
                            .child("停止")
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.cancel_generation(cx)
                            })),
                    ),
            );
        }

        let send_button = if streaming_here {
            let mut button = div()
                .id("composer-stop")
                .size(px(34.))
                .flex_none()
                .mb(px(2.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(theme::accent())
                .text_color(theme::bg())
                .text_size(px(11.))
                .cursor_pointer()
                .child("■");
            button = button.on_click(
                cx.listener(|this, _: &gpui::ClickEvent, _, cx| this.cancel_generation(cx)),
            );
            button
        } else {
            let mut button = div()
                .id("composer-send")
                .size(px(34.))
                .flex_none()
                .mb(px(2.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(theme::text())
                .text_color(theme::bg())
                .text_size(px(16.))
                .cursor_pointer()
                .hover(|s| s.bg(gpui::rgb(0x1f1b17)))
                .child("↑");
            button =
                button.on_click(cx.listener(|this, _: &gpui::ClickEvent, window, cx| {
                    this.send_composer(window, cx)
                }));
            button
        };

        area.child(
            div()
                .max_w(px(768.))
                .mx_auto()
                .w_full()
                .flex()
                .items_end()
                .gap(px(8.))
                .child(div().flex_1().min_w(px(0.)).child(self.composer.clone()))
                .child(send_button),
        )
        .child(
            div()
                .max_w(px(768.))
                .mx_auto()
                .w_full()
                .flex()
                .justify_between()
                .mt(px(5.))
                .px(px(2.))
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(theme::faint())
                        .child("Enterで送信 · Shift+Enterで改行"),
                )
                .child(
                    div()
                        .id("delete-conversation")
                        .text_size(px(10.))
                        .text_color(theme::faint())
                        .cursor_pointer()
                        .hover(|s| s.text_color(theme::muted()))
                        .child("この会話を削除")
                        .on_click(cx.listener(|this, _: &gpui::ClickEvent, window, cx| {
                            this.confirm_delete_conversation(window, cx)
                        })),
                ),
        )
    }

    fn render_panel(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> gpui::Div {
        let Some(conversation) = self.active_conversation().cloned() else {
            return div();
        };
        let character = self.active_character().cloned();
        let character_id = character.as_ref().map(|c| c.id.clone());
        let scenario = if conversation.scenario.trim().is_empty() {
            "未設定".to_string()
        } else {
            conversation.scenario.clone()
        };
        let scene = if conversation.scene.trim().is_empty() {
            "未設定".to_string()
        } else {
            conversation.scene.clone()
        };
        let notes = if conversation.notes.trim().is_empty() {
            "未設定".to_string()
        } else {
            conversation.notes.clone()
        };

        let mut body = div()
            .flex_1()
            .min_h(px(0.))
            .id("panel-scroll")
            .overflow_y_scroll()
            .p(px(16.))
            .flex()
            .flex_col()
            .gap(px(20.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(5.))
                    .child(ui::section_title("シナリオ"))
                    .child(
                        div()
                            .font_family(theme::serif_font())
                            .text_size(px(12.5))
                            .line_height(px(22.))
                            .text_color(if conversation.scenario.trim().is_empty() {
                                theme::faint()
                            } else {
                                theme::text()
                            })
                            .child(scenario),
                    )
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(theme::faint())
                            .child("現在の場面"),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(21.))
                            .text_color(if conversation.scene.trim().is_empty() {
                                theme::faint()
                            } else {
                                theme::muted()
                            })
                            .child(scene),
                    )
                    .child(
                        div()
                            .id("panel-edit-scene")
                            .text_size(px(11.))
                            .text_color(theme::muted())
                            .cursor_pointer()
                            .hover(|s| s.text_color(theme::text()))
                            .child("場面を編集")
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, window, cx| {
                                this.open_scene(window, cx)
                            })),
                    ),
            )
            .child(ui::divider())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(9.))
                    .child(ui::section_title("キャラクター"))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(9.))
                            .child(match &character {
                                Some(character) => {
                                    ui::avatar(&character.name, &character.accent, 30.)
                                }
                                None => ui::narrator_avatar(30.),
                            })
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(1.))
                                    .min_w(px(0.))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(
                                                character
                                                    .as_ref()
                                                    .map(|c| c.name.clone())
                                                    .unwrap_or_else(|| "語り手".into()),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(theme::muted())
                                            .overflow_hidden()
                                            .truncate()
                                            .child(
                                                character
                                                    .as_ref()
                                                    .map(|c| c.description.clone())
                                                    .unwrap_or_default(),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("panel-edit-character")
                            .text_size(px(11.))
                            .text_color(theme::muted())
                            .cursor_pointer()
                            .hover(|s| s.text_color(theme::text()))
                            .child(if character.is_some() {
                                "プロフィールを編集"
                            } else {
                                "キャラクターを作る"
                            })
                            .on_click(cx.listener(
                                move |this, _: &gpui::ClickEvent, window, cx| {
                                    this.open_character(character_id.clone(), window, cx)
                                },
                            )),
                    ),
            )
            .child(ui::divider())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(5.))
                    .child(ui::section_title("覚えておくこと"))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .line_height(px(20.))
                            .text_color(if conversation.notes.trim().is_empty() {
                                theme::faint()
                            } else {
                                theme::muted()
                            })
                            .child(notes),
                    ),
            );

        if let Some(character) = &character {
            if !character.speech_style.trim().is_empty() {
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(ui::section_title("話し方"))
                        .child(
                            div()
                                .text_size(px(11.5))
                                .line_height(px(20.))
                                .text_color(theme::muted())
                                .child(character.speech_style.clone()),
                        ),
                );
            }
        }

        div()
            .w(px(300.))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .bg(theme::bg())
            .border_l_1()
            .border_color(theme::line())
            .child(
                div()
                    .h(px(60.))
                    .flex_none()
                    .px(px(16.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(theme::line())
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("この物語"),
                    )
                    .child(ui::icon_button("×", 26.).id("close-panel").on_click(
                        cx.listener(|this, _: &gpui::ClickEvent, _, cx| this.close_panel(cx)),
                    )),
            )
            .child(body)
    }
}

fn mode_button(
    label: &'static str,
    active: bool,
    id: &'static str,
    cx: &mut gpui::Context<HomerApp>,
) -> gpui::AnyElement {
    let mode = if id == "mode-story" {
        "story"
    } else {
        "messenger"
    };
    div()
        .id(id)
        .h(px(24.))
        .px(px(10.))
        .flex()
        .items_center()
        .rounded(px(5.))
        .text_size(px(11.5))
        .cursor_pointer()
        .text_color(if active {
            theme::text()
        } else {
            theme::muted()
        })
        .when(active, |d| {
            d.bg(theme::surface()).shadow_sm().text_color(theme::text())
        })
        .child(label)
        .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| this.set_mode(mode, cx)))
        .into_any_element()
}

pub(crate) fn render_message(
    message: &Message,
    character: Option<&Character>,
    mode: &str,
    show_metrics: bool,
) -> gpui::Div {
    if message.role == "user" {
        return div().w_full().flex().justify_end().py(px(6.)).child(
            div()
                .flex()
                .flex_col()
                .items_end()
                .gap(px(3.))
                .max_w(px(560.))
                .child(
                    div()
                        .bg(gpui::rgb(theme::USER_BUBBLE))
                        .border_1()
                        .border_color(gpui::rgb(theme::USER_BUBBLE_LINE))
                        .rounded_tl(px(14.))
                        .rounded_tr(px(14.))
                        .rounded_bl(px(14.))
                        .rounded_br(px(3.))
                        .px(px(13.))
                        .py(px(8.))
                        .text_size(px(14.5))
                        .line_height(px(25.))
                        .child(message.content.clone()),
                )
                .child(
                    div()
                        .text_size(px(9.5))
                        .text_color(theme::faint())
                        .child(theme::format_time(message.created_at)),
                ),
        );
    }

    let name = character
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "語り手".into());
    let mut meta = div()
        .flex()
        .items_center()
        .gap(px(7.))
        .mb(px(6.))
        .min_h(px(22.))
        .child(match character {
            Some(character) => ui::avatar(&character.name, &character.accent, 22.),
            None => ui::narrator_avatar(22.),
        })
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(name.clone()),
        )
        .child(
            div()
                .text_size(px(9.5))
                .text_color(theme::faint())
                .child(theme::format_time(message.created_at)),
        );
    if let Some(meta_data) = &message.meta {
        if meta_data.stopped {
            meta = meta.child(tag("中断"));
        }
        if meta_data.incomplete {
            meta = meta.child(tag("未完了"));
        }
    }

    let mut content = div()
        .pl(px(29.))
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(segments_view(&message.content, mode, &name));

    if show_metrics {
        if let Some(metrics) = message.meta.as_ref().and_then(|meta| meta.metrics.as_ref()) {
            let mut line = format!(
                "TTFT {:.2}s · {:.1} tok/s · {:.1}s · {} tok",
                metrics.ttft_ms as f64 / 1000.,
                metrics.tokens_per_sec,
                metrics.duration_ms as f64 / 1000.,
                metrics.estimated_tokens
            );
            if metrics.trimmed_messages > 0 {
                line.push_str(&format!(" · 履歴 {} 件省略", metrics.trimmed_messages));
            }
            content = content.child(
                div()
                    .text_size(px(9.5))
                    .text_color(theme::faint())
                    .font_family("Menlo")
                    .child(line),
            );
        }
    }

    div()
        .w_full()
        .flex()
        .flex_col()
        .py(px(10.))
        .child(meta)
        .child(content)
}

fn render_streaming(mode: &str, character: Option<&Character>, text: &str) -> gpui::Div {
    let name = character
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "語り手".into());
    let mut content = div().pl(px(29.)).flex().flex_col().gap(px(4.));
    if text.is_empty() {
        content = content.child(
            div()
                .py(px(4.))
                .text_size(px(12.))
                .text_color(theme::faint())
                .child("…"),
        );
    } else {
        content = content.child(segments_view(text, mode, &name));
    }
    div()
        .w_full()
        .flex()
        .flex_col()
        .py(px(10.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(7.))
                .mb(px(6.))
                .child(match character {
                    Some(character) => ui::avatar(&character.name, &character.accent, 22.),
                    None => ui::narrator_avatar(22.),
                })
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(name.clone()),
                ),
        )
        .child(content)
}

fn segments_view(content: &str, mode: &str, character_name: &str) -> gpui::Div {
    let segments = visible_segments(
        &homer_core::segment_text(content, Some(character_name)),
        mode,
    );
    let mut body = div().flex().flex_col().gap(px(4.));
    for segment in segments {
        let is_dialogue = segment.kind == SegmentType::Dialogue;
        body = body.child(
            div()
                .text_size(px(if is_dialogue { 15. } else { 14.5 }))
                .line_height(px(if is_dialogue { 27. } else { 28. }))
                .when(!is_dialogue, |d| {
                    d.font_family(theme::serif_font())
                        .text_color(gpui::rgb(0x655d51))
                })
                .child(segment.text),
        );
    }
    body
}

fn tag(label: &'static str) -> gpui::Div {
    div()
        .px(px(5.))
        .py(px(1.))
        .border_1()
        .border_color(theme::line())
        .rounded(px(4.))
        .text_size(px(9.))
        .text_color(theme::faint())
        .child(label)
}

fn kind_label(kind: &str) -> &'static str {
    match kind {
        "network" => "接続",
        "http" => "サーバー応答",
        "persistence" => "保存",
        "config" => "設定",
        "empty" => "空の応答",
        _ => "生成",
    }
}

fn empty_list(
    message: &'static str,
    action: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> gpui::Div {
    div()
        .py(px(28.))
        .px(px(12.))
        .flex()
        .flex_col()
        .items_center()
        .gap(px(10.))
        .text_size(px(12.))
        .text_color(theme::faint())
        .child(message)
        .child(
            div()
                .id(SharedString::from(format!("empty-{action}")))
                .text_size(px(12.))
                .text_color(theme::muted())
                .cursor_pointer()
                .hover(|s| s.text_color(theme::text()))
                .child(action)
                .on_click(on_click),
        )
}
