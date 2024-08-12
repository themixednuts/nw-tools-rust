use crate::events::EventBus;
use std::sync::{LazyLock, OnceLock};
use tokio_util::sync::CancellationToken;
use utils::lumberyard::LumberyardSource;

static APP: OnceLock<App> = OnceLock::new();

#[derive(Debug, Default)]
pub struct App {
    state: AppState,
    bus: EventBus,
    pub cancel: CancellationToken,
}

#[derive(Debug, Default)]
struct AppState {
    map: LumberyardSource,
}

impl App {
    pub fn init() -> &'static Self {
        APP.get_or_init(|| App::default())
    }
    pub fn handle() -> &'static Self {
        APP.get().expect("App wasn't initialized")
    }
    pub fn event(&self) {}
}
