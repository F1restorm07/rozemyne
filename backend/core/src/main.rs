use backend_core::configuration::ConfigPlugin;
use bevy_app::App;

fn main() {
    App::new()
        .add_plugins((
            // @todo: allow for custom config paths (+ add defualts)
            // @todo: check `Settings` for config path (+ fallback default)
            ConfigPlugin::new("./test.toml"),
            backend_core::SetupPlugins
        ))
        .run();
}
