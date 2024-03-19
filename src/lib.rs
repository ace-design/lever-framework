pub use lever_core;

pub async fn start_server(setup: &lever_core::Setup) {
    lever_core::start_server(setup).await
}
