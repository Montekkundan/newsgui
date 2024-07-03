mod headlines;

use std::{
    sync::mpsc::{channel, sync_channel},
    thread,
};

use eframe::{
    egui::{
        CentralPanel, CtxRef, Hyperlink, Label, ScrollArea, Separator, TextStyle,
        TopBottomPanel, Ui, Visuals,
    },
    epi::App,
};
pub use headlines::{Headlines, Msg, NewsCardData, PADDING};
use newsapi::NewsAPI;

impl App for Headlines {
    fn setup(
        &mut self,
        ctx: &eframe::egui::CtxRef,
        _frame: &mut eframe::epi::Frame<'_>,
        storage: Option<&dyn eframe::epi::Storage>,
    ) {
        if let Some(storage) = storage {
            self.config = eframe::epi::get_value(storage, "headlines").unwrap_or_default();
        }

        let (mut news_tx, news_rx) = channel();
        let (app_tx, app_rx) = sync_channel(1);

        self.app_tx = Some(app_tx.clone());

        self.news_rx = Some(news_rx);

        #[cfg(not(target_arch = "wasm32"))]
        thread::spawn(move || {
            loop {
                match app_rx.recv() {
                    Ok(Msg::Refresh) => {
                        fetch_news(&mut news_tx);
                    }
                    Err(e) => {
                        tracing::error!("failed receiving msg: {}", e);
                    }
                }
            }
        });

        // Send initial refresh message to fetch data on startup
        if let Err(e) = app_tx.send(Msg::Refresh) {
            tracing::error!("Failed to send initial refresh message: {}", e);
        }

        #[cfg(target_arch = "wasm32")]
        gloo_timers::callback::Timeout::new(10, move || {
            wasm_bindgen_futures::spawn_local(async {
                fetch_web(news_tx).await;
            });
        })
        .forget();

        #[cfg(target_arch = "wasm32")]
        gloo_timers::callback::Interval::new(500, move || match app_rx.try_recv() {
            Ok(Msg::Refresh) => {
                wasm_bindgen_futures::spawn_local(fetch_web(news_tx.clone()));
            }
            Err(e) => {
                tracing::error!("failed receiving msg: {}", e);
            }
        })
        .forget();

        self.configure_fonts(ctx);
    }

    fn update(&mut self, ctx: &eframe::egui::CtxRef, frame: &mut eframe::epi::Frame<'_>) {
        ctx.request_repaint();

        if self.config.dark_mode {
            ctx.set_visuals(Visuals::dark());
        } else {
            ctx.set_visuals(Visuals::light());
        }

        self.preload_articles();

        self.render_top_panel(ctx, frame);
        CentralPanel::default().show(ctx, |ui| {
            if self.articles.is_empty() {
                ui.vertical_centered_justified(|ui| {
                    ui.heading("Loading ⌛");
                });
            } else {
                render_header(ui);
                ScrollArea::auto_sized().show(ui, |ui| {
                    self.render_news_cards(ui);
                });
                render_footer(ctx);
            }
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::epi::Storage) {
        eframe::epi::set_value(storage, "headlines", &self.config);
    }

    fn name(&self) -> &str {
        "Headlines"
    }
}

fn fetch_news(news_tx: &mut std::sync::mpsc::Sender<NewsCardData>) {
    tracing::info!("Attempting to fetch news...");
    match NewsAPI::new().fetch() {
        Ok(articles) => {
            tracing::info!("Successfully fetched news.");
            for a in articles.iter() {
                let news = NewsCardData {
                    title: a.title().to_string(),
                    url: a.content().to_string(), // Update to use the content as URL for this example
                    desc: a.content().to_string(),
                    source: a.source().to_string(),
                };
                if let Err(e) = news_tx.send(news) {
                    tracing::error!("Error sending news data: {}", e);
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed fetching news: {:?}", e);
        }
    }
}



#[cfg(target_arch = "wasm32")]
async fn fetch_web(news_tx: std::sync::mpsc::Sender<NewsCardData>) {
    if let Ok(response) = NewsAPI::new().fetch_web().await {
        let resp_articles = response.articles();
        for a in resp_articles.iter() {
            let news = NewsCardData {
                title: a.title().to_string(),
                url: a.content().to_string(), // Update to use the content as URL for this example
                desc: a.content().to_string(),
                source: a.source().to_string(),
            };
            if let Err(e) = news_tx.send(news) {
                tracing::error!("Error sending news data: {}", e);
            }
        }
    } else {
        tracing::error!("failed fetching news");
    }
}

fn render_footer(ctx: &CtxRef) {
    TopBottomPanel::bottom("footer").show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(10.);
            ui.add(Label::new("API source: localhost:8080").monospace());
            ui.add(
                Hyperlink::new("https://www.montek.dev")
                    .text("Made by Montek")
                    .text_style(TextStyle::Monospace),
            );
            ui.add_space(10.);
        })
    });
}

fn render_header(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.heading("Top Headlines");
    });
    ui.add_space(PADDING);
    let sep = Separator::default().spacing(20.);
    ui.add(sep);
}

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn main_web(canvas_id: &str) {
    let headlines = Headlines::new();
    tracing_wasm::set_as_global_default();
    eframe::start_web(canvas_id, Box::new(headlines));
}
