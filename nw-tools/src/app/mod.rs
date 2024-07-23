use std::sync::OnceLock;
use tokio_util::sync::CancellationToken;

static APP: OnceLock<App> = OnceLock::new();

#[derive(Debug, Default)]
pub struct App {
    state: AppState,
    pub notify: CancellationToken,
}

#[derive(Debug, Default)]
struct AppState {}

impl App {
    pub fn init() -> &'static Self {
        APP.get_or_init(App::default)
    }
}
