use std::sync::Arc;

use smn_web_core::{plugins::plugin_static::PluginStatic, systems::{sys_core::run_server, sys_plugin::PluginManager}};

pub mod plugins;

#[tokio::main]
async fn main() {

    let mut manager = PluginManager::new();
    let mut components = plugins::plugin_components::PluginComponents::new();

    components.register(plugins::plugin_components::components::comp_header::CompHeader);
    components.register_simple("./components/underConstruction.html");
    components.register_simple("./components/background.html");

    manager.apply_plugin(Box::new(components));
    manager.apply_plugin(Box::new(PluginStatic::new(true, vec!["html".to_string(), "pdf".to_string(), "svg".to_string()])));
    manager.init_plugins().await;
    let manager = Arc::new(manager);

    // Run the server on port 8000.
    run_server(8000, false, manager).await;
}
