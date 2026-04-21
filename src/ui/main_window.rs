use gpui::{AppContext, Color, View, ViewContext, WindowContext, WindowOptions, Size, vstack, hstack, button, label};

pub struct MainWindow {
    view: gpui::View<MainWindowView>,
}

pub struct MainWindowView {
    app_state: crate::app::AppState,
}

impl MainWindow {
    pub fn new(cx: &mut AppContext) -> Self {
        let view = cx.new_view(|cx| MainWindowView {
            app_state: crate::app::AppState::new(),
        });
        Self {
            view,
        }
    }
}

impl View for MainWindow {
    fn ui_name() -> &'static str {
        "MainWindow"
    }

    fn show(&mut self, cx: &mut WindowContext) {
        cx.open_window(WindowOptions {
            title: "DataGuard",
            size: Size::new(1000.0, 800.0),
            ..Default::default()
        });
    }
}

impl View for MainWindowView {
    fn ui_name() -> &'static str {
        "MainWindowView"
    }

    fn render(&mut self, cx: &mut ViewContext<Self>) {
        let mut ui = cx.new_ui();
        ui.add(|cx| {
            vstack((
                hstack((
                    button("Open Parquet File").on_press(|cx| {
                        // TODO: Implement file open dialog
                    }),
                ))
                .padding(16.0)
                .background(Color::rgb(240, 240, 240)),
                
                // TODO: Add file info and diagnosis report display
                // TODO: Add data preview table
            ))
            .fill()
        });
        ui.build();
    }
}
