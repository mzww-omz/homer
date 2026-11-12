use crate::app::{HomerApp, Modal};
use crate::text_area::TextArea;
use crate::{theme, ui};
use gpui::{div, prelude::*, px, relative, ElementId, Entity, FontWeight, Window};

const ACCENTS: [&str; 6] = [
    "#7d6f5c", "#8a5a4a", "#5f6f5a", "#5a6579", "#7b5f78", "#8c7350",
];

impl HomerApp {
    pub(crate) fn render_modal(
        &mut self,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) -> gpui::Div {
        match self.modal {
            Modal::Character => self.character_modal(window, cx),
            Modal::NewConversation => self.new_conversation_modal(window, cx),
            Modal::Settings => self.settings_modal(window, cx),
            Modal::Scene => self.scene_modal(window, cx),
            Modal::None => div(),
        }
    }

    fn character_modal(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> gpui::Div {
        let editing = self.char_form.id.is_some();
        let title = if editing {
            "キャラクターを編集"
        } else {
            "新しいキャラクター"
        };
        let preview_name = self.char_form.name.read(cx).content();
        let preview_name = if preview_name.trim().is_empty() {
            "?".to_string()
        } else {
            preview_name
        };
        let accent = self.char_form.accent.clone();
        let saving = self.char_form.saving;

        let mut accents = div().flex().items_center().gap(px(8.));
        for candidate in ACCENTS {
            let selected = accent == candidate;
            let color = theme::parse_hex(candidate);
            let value = candidate.to_string();
            accents = accents.child(
                div()
                    .id(ElementId::Name(format!("accent-{candidate}").into()))
                    .size(px(20.))
                    .rounded_full()
                    .bg(color)
                    .border_2()
                    .border_color(if selected { theme::text() } else { theme::bg() })
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                        this.char_form.accent = value.clone();
                        cx.notify();
                    })),
            );
        }

        let body = div()
            .flex_1()
            .id("character-modal-scroll")
            .overflow_y_scroll()
            .px(px(20.))
            .pb(px(20.))
            .flex()
            .flex_col()
            .gap(px(13.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(ui::avatar(&preview_name, &accent, 44.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(theme::faint())
                                    .child("アクセント"),
                            )
                            .child(accents),
                    ),
            )
            .child(field("名前", self.char_form.name.clone()))
            .child(field("どんな人？", self.char_form.description.clone()))
            .child(field("話し方", self.char_form.speech_style.clone()))
            .child(
                div()
                    .flex()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex_1()
                            .child(field("一人称", self.char_form.first_person.clone())),
                    )
                    .child(div().flex_1().child(field(
                        "ユーザーとの関係",
                        self.char_form.relationship.clone(),
                    ))),
            )
            .child(
                div()
                    .flex()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex_1()
                            .child(field("物語上の役割", self.char_form.role.clone())),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(field("性格", self.char_form.personality.clone())),
                    ),
            )
            .child(field("発話例", self.char_form.example_dialogue.clone()))
            .child(field("追加設定", self.char_form.extra.clone()))
            .child(
                div()
                    .text_size(px(10.5))
                    .line_height(px(18.))
                    .text_color(theme::faint())
                    .child("名前以外はすべて任意です。短い説明だけでも始められます。"),
            );

        let mut footer = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .px(px(20.))
            .py(px(12.))
            .border_t_1()
            .border_color(theme::line())
            .bg(theme::surface_2());
        if editing {
            footer = footer.child(
                ui::button("削除", false)
                    .id("delete-character")
                    .text_color(theme::danger())
                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, window, cx| {
                        this.delete_character(window, cx)
                    })),
            );
        }
        footer = footer
            .child(div().flex_1())
            .child(
                ui::button("キャンセル", false)
                    .id("cancel-character")
                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, window, cx| {
                        this.modal = Modal::None;
                        window.focus(&this.composer_focus(cx));
                        cx.notify();
                    })),
            )
            .child(
                ui::button(if saving { "保存中…" } else { "保存" }, true)
                    .id("save-character")
                    .on_click(
                        cx.listener(|this, _: &gpui::ClickEvent, _, cx| this.save_character(cx)),
                    ),
            );

        modal_shell(title, 620., "character-modal-close", body, footer, cx)
    }

    fn new_conversation_modal(
        &mut self,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) -> gpui::Div {
        let mut characters = div().flex().flex_wrap().gap(px(6.));
        let none_selected = self.new_form.character_id.is_none();
        characters = characters.child(
            div()
                .id("new-char-none")
                .flex()
                .items_center()
                .gap(px(5.))
                .h(px(28.))
                .px(px(9.))
                .rounded(px(6.))
                .border_1()
                .border_color(if none_selected {
                    theme::text()
                } else {
                    theme::line()
                })
                .bg(if none_selected {
                    theme::surface()
                } else {
                    theme::bg()
                })
                .text_size(px(11.5))
                .text_color(if none_selected {
                    theme::text()
                } else {
                    theme::muted()
                })
                .cursor_pointer()
                .child("語り手だけ")
                .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                    this.new_form.character_id = None;
                    cx.notify();
                })),
        );
        for character in self.characters.clone() {
            let selected = self.new_form.character_id.as_deref() == Some(character.id.as_str());
            let id = character.id.clone();
            characters = characters.child(
                div()
                    .id(ElementId::Name(format!("new-char-{}", character.id).into()))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(if selected {
                        theme::text()
                    } else {
                        theme::line()
                    })
                    .bg(if selected {
                        theme::surface()
                    } else {
                        theme::bg()
                    })
                    .cursor_pointer()
                    .child(ui::avatar(&character.name, &character.accent, 18.))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(if selected {
                                theme::text()
                            } else {
                                theme::muted()
                            })
                            .child(character.name.clone()),
                    )
                    .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                        this.new_form.character_id = Some(id.clone());
                        cx.notify();
                    })),
            );
        }

        let creating = self.new_form.saving;
        let mode = self.new_form.mode.clone();
        let body = div()
            .flex_1()
            .id("new-conversation-scroll")
            .overflow_y_scroll()
            .px(px(20.))
            .pb(px(20.))
            .flex()
            .flex_col()
            .gap(px(15.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(7.))
                    .child(ui::section_title("誰と話す？"))
                    .child(characters)
                    .child(
                        div()
                            .id("new-conversation-add-character")
                            .text_size(px(11.))
                            .text_color(theme::muted())
                            .cursor_pointer()
                            .hover(|s| s.text_color(theme::text()))
                            .child("＋ 新しいキャラクターを作る")
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, window, cx| {
                                this.modal = Modal::None;
                                this.open_character(None, window, cx);
                            })),
                    ),
            )
            .child(field("タイトル（任意）", self.new_form.title.clone()))
            .child(field("場面・シナリオ", self.new_form.scenario.clone()))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(7.))
                    .child(ui::section_title("表示形式"))
                    .child(
                        div()
                            .flex()
                            .gap(px(7.))
                            .child(mode_chip("物語", mode == "story", "new-mode-story", cx))
                            .child(mode_chip(
                                "メッセンジャー",
                                mode == "messenger",
                                "new-mode-messenger",
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(theme::faint())
                            .child("あとから会話画面で切り替えられます。"),
                    ),
            );

        let footer = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .px(px(20.))
            .py(px(12.))
            .border_t_1()
            .border_color(theme::line())
            .bg(theme::surface_2())
            .child(div().flex_1())
            .child(
                ui::button("キャンセル", false)
                    .id("cancel-new-conversation")
                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, window, cx| {
                        this.modal = Modal::None;
                        window.focus(&this.composer_focus(cx));
                        cx.notify();
                    })),
            )
            .child(
                ui::button(
                    if creating {
                        "作成中…"
                    } else {
                        "始める"
                    },
                    true,
                )
                .id("create-conversation")
                .on_click(
                    cx.listener(|this, _: &gpui::ClickEvent, _, cx| this.save_new_conversation(cx)),
                ),
            );

        let _ = window;
        modal_shell(
            "新しい会話",
            540.,
            "new-conversation-close",
            body,
            footer,
            cx,
        )
    }

    fn settings_modal(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> gpui::Div {
        let busy = self.settings_form.busy;
        let status = self.settings_form.status.clone();
        let key_set = self.settings_form.key_set;
        let show_metrics = self.settings_form.show_metrics;
        let current_model = self.settings_form.model.read(cx).content();
        let models = self.settings_form.models.clone();

        let mut body = div()
            .flex_1()
            .id("settings-modal-scroll")
            .overflow_y_scroll()
            .px(px(20.))
            .pb(px(20.))
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(ui::section_title("接続"))
            .child(field("エンドポイント", self.settings_form.base_url.clone()))
            .child(
                div()
                    .text_size(px(10.5))
                    .line_height(px(18.))
                    .text_color(theme::faint())
                    .child("LM Studio: http://localhost:1234/v1 ／ Ollama: http://localhost:11434/v1"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(5.))
                    .child(ui::label("APIキー（任意）"))
                    .child(self.settings_form.api_key.clone())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(theme::faint())
                                    .child(if key_set {
                                        "設定済み。変更しない場合は空欄のまま。"
                                    } else {
                                        "ローカルサーバーでは不要です。セッション中のみ保持し、保存しません。"
                                    }),
                            )
                            .when(key_set, |row| {
                                row.child(
                                    div()
                                        .id("clear-api-key")
                                        .text_size(px(10.5))
                                        .text_color(theme::muted())
                                        .cursor_pointer()
                                        .hover(|s| s.text_color(theme::danger()))
                                        .child("消去")
                                        .on_click(cx.listener(
                                            |this, _: &gpui::ClickEvent, _, cx| {
                                                this.clear_api_key(cx)
                                            },
                                        )),
                                )
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap(px(8.))
                    .child(div().flex_1().child(field("モデル", self.settings_form.model.clone())))
                    .child(
                        ui::button(if busy { "確認中…" } else { "接続確認" }, false)
                            .id("fetch-models")
                            .mb(px(0.))
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.fetch_models(cx)
                            })),
                    ),
            );

        if let Some((ok, message)) = status {
            body = body.child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(7.))
                    .text_size(px(11.))
                    .line_height(px(18.))
                    .text_color(if ok {
                        gpui::rgb(0x4e6a4d)
                    } else {
                        theme::danger()
                    })
                    .child(
                        div()
                            .flex_none()
                            .mt(px(6.))
                            .size(px(6.))
                            .rounded_full()
                            .bg(if ok {
                                gpui::rgb(0x5b7a5a)
                            } else {
                                theme::danger()
                            }),
                    )
                    .child(div().flex_1().child(message)),
            );
        }

        if !models.is_empty() {
            let mut list = div()
                .id("models-list")
                .max_h(px(112.))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap(px(1.))
                .p(px(4.))
                .border_1()
                .border_color(theme::line())
                .rounded(px(7.))
                .bg(theme::surface());
            for model in models {
                let selected = model == current_model;
                let value = model.clone();
                list = list.child(
                    div()
                        .id(ElementId::Name(format!("model-{model}").into()))
                        .px(px(8.))
                        .py(px(5.))
                        .rounded(px(4.))
                        .text_size(px(11.5))
                        .text_color(if selected {
                            theme::text()
                        } else {
                            theme::muted()
                        })
                        .when(selected, |row| row.bg(theme::surface_2()))
                        .cursor_pointer()
                        .hover(|s| s.bg(theme::surface_2()))
                        .child(model)
                        .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                            this.settings_form
                                .model
                                .update(cx, |input, cx| input.set_content(&value, cx));
                            cx.notify();
                        })),
                );
            }
            body = body
                .child(ui::section_title("利用できるモデル"))
                .child(list);
        }

        body = body
            .child(div().h(px(1.)).w_full().bg(theme::line()))
            .child(ui::section_title("生成"))
            .child(
                div()
                    .flex()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex_1()
                            .child(field("温度", self.settings_form.temperature.clone())),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(field("最大トークン", self.settings_form.max_tokens.clone())),
                    )
                    .child(div().flex_1().child(field(
                        "コンテキスト上限",
                        self.settings_form.context_budget.clone(),
                    ))),
            )
            .child(
                div()
                    .text_size(px(10.5))
                    .line_height(px(18.))
                    .text_color(theme::faint())
                    .child("温度は0〜2。コンテキスト上限はプロンプト全体の目安（トークン）です。"),
            )
            .child(
                div()
                    .id("toggle-show-metrics")
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .cursor_pointer()
                    .child(
                        div()
                            .size(px(16.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(if show_metrics {
                                theme::text()
                            } else {
                                theme::line_strong()
                            })
                            .bg(if show_metrics {
                                theme::text()
                            } else {
                                theme::surface()
                            })
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(10.))
                            .text_color(theme::bg())
                            .child(if show_metrics { "✓" } else { "" }),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(theme::text())
                                    .child("計測情報を表示"),
                            )
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(theme::faint())
                                    .child("TTFT、生成速度、コンテキスト使用量（開発用）"),
                            ),
                    )
                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                        this.settings_form.show_metrics = !this.settings_form.show_metrics;
                        cx.notify();
                    })),
            );

        let footer =
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .px(px(20.))
                .py(px(12.))
                .border_t_1()
                .border_color(theme::line())
                .bg(theme::surface_2())
                .child(
                    div()
                        .text_size(px(9.5))
                        .text_color(theme::faint())
                        .child("設定はこの端末に保存されます"),
                )
                .child(div().flex_1())
                .child(
                    ui::button("キャンセル", false)
                        .id("cancel-settings")
                        .on_click(cx.listener(|this, _: &gpui::ClickEvent, window, cx| {
                            this.modal = Modal::None;
                            window.focus(&this.composer_focus(cx));
                            cx.notify();
                        })),
                )
                .child(ui::button("保存", true).id("save-settings").on_click(
                    cx.listener(|this, _: &gpui::ClickEvent, _, cx| this.save_settings(cx)),
                ));

        let _ = window;
        modal_shell("設定", 580., "settings-modal-close", body, footer, cx)
    }

    fn scene_modal(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> gpui::Div {
        let saving = self.scene_form.saving;
        let body = div()
            .flex_1()
            .id("scene-modal-scroll")
            .overflow_y_scroll()
            .px(px(20.))
            .pb(px(20.))
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(field("タイトル", self.scene_form.title.clone()))
            .child(field("シナリオ", self.scene_form.scenario.clone()))
            .child(field("現在の場面", self.scene_form.scene.clone()))
            .child(field("覚えておくこと", self.scene_form.notes.clone()))
            .child(
                div()
                    .text_size(px(10.5))
                    .line_height(px(18.))
                    .text_color(theme::faint())
                    .child(
                        "「覚えておくこと」はプロンプトに含まれます。長くしすぎないでください。",
                    ),
            );

        let footer = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .px(px(20.))
            .py(px(12.))
            .border_t_1()
            .border_color(theme::line())
            .bg(theme::surface_2())
            .child(div().flex_1())
            .child(
                ui::button("キャンセル", false)
                    .id("cancel-scene")
                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, window, cx| {
                        this.modal = Modal::None;
                        window.focus(&this.composer_focus(cx));
                        cx.notify();
                    })),
            )
            .child(
                ui::button(if saving { "保存中…" } else { "保存" }, true)
                    .id("save-scene")
                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| this.save_scene(cx))),
            );

        modal_shell("場面と記憶", 560., "scene-modal-close", body, footer, cx)
    }
}

fn modal_shell(
    title: &'static str,
    width: f32,
    close_id: &'static str,
    body: impl gpui::IntoElement + 'static,
    footer: impl gpui::IntoElement + 'static,
    cx: &mut gpui::Context<HomerApp>,
) -> gpui::Div {
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .p(px(24.))
        .bg(gpui::rgba(0x2c261f52))
        .child(
            div()
                .w(px(width))
                .max_h(relative(0.9))
                .flex()
                .flex_col()
                .bg(theme::bg())
                .border_1()
                .border_color(theme::line())
                .rounded(px(12.))
                .shadow_lg()
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px(px(20.))
                        .pt(px(16.))
                        .pb(px(10.))
                        .child(
                            div()
                                .text_size(px(14.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(title),
                        )
                        .child(ui::icon_button("×", 26.).id(close_id).on_click(cx.listener(
                            |this, _: &gpui::ClickEvent, window, cx| {
                                this.modal = Modal::None;
                                window.focus(&this.composer_focus(cx));
                                cx.notify();
                            },
                        ))),
                )
                .child(body)
                .child(footer),
        )
}

fn field(label: &'static str, entity: Entity<TextArea>) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(px(0.))
        .child(ui::label(label))
        .child(entity)
}

fn mode_chip(
    label: &'static str,
    active: bool,
    id: &'static str,
    cx: &mut gpui::Context<HomerApp>,
) -> gpui::AnyElement {
    let mode = if id == "new-mode-story" {
        "story"
    } else {
        "messenger"
    };
    div()
        .id(id)
        .h(px(28.))
        .px(px(11.))
        .flex()
        .items_center()
        .gap(px(5.))
        .rounded(px(6.))
        .border_1()
        .border_color(if active { theme::text() } else { theme::line() })
        .bg(if active {
            theme::surface()
        } else {
            theme::bg()
        })
        .text_size(px(11.5))
        .text_color(if active {
            theme::text()
        } else {
            theme::muted()
        })
        .cursor_pointer()
        .child(label)
        .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
            this.new_form.mode = mode.to_string();
            cx.notify();
        }))
        .into_any_element()
}
