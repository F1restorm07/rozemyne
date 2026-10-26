#![allow(unused)] // to silence all the `unused ...` warnings (remove later)
#![allow(dead_code)] // to silence all the `... are never read` warnings (remove later)

use backend_types::lua::ScriptAsset;
use bevy_asset::AssetApp;

use self::lua::ScriptAssetLoader;

pub mod cli;
pub mod files;
pub mod source;
pub mod storage;
pub mod configuration;
pub mod lua;

pub struct SetupPlugins;

impl bevy_app::PluginGroup for SetupPlugins {
    fn build(self) -> bevy_app::PluginGroupBuilder {
        bevy_app::PluginGroupBuilder::start::<Self>()
            .add(cli::CliPlugin)
            .add(source::DataSourcePlugin)
            .add(storage::ArchivalPlugin)
            .add(SchedulePlugin)
            // @todo: customize this
            .add(bevy_log::LogPlugin {
                level: bevy_log::Level::INFO,
                ..Default::default()
            })
            .add(bevy_app::TaskPoolPlugin::default())
            .add(bevy_time::TimePlugin)
            .add(bevy_asset::AssetPlugin {
                file_path: "../../assets".to_string(),
                ..Default::default()
            })
            .add(ScriptingPlugin)
    }
}

pub struct SchedulePlugin;

impl bevy_app::Plugin for SchedulePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let settings = app.world().get_resource::<configuration::Settings>().unwrap();
        let mut archive_update_timer = bevy_time::Timer::new(settings.archive_frequency(), bevy_time::TimerMode::Repeating);

        // mostly ripped from the `ScheduleRunnerPlugin` in `bevy_app`
        app.set_runner(move |mut app: bevy_app::App| {
            let plugin_state = app.plugins_state();
            if plugin_state != bevy_app::PluginsState::Cleaned {
                while app.plugins_state() == bevy_app::PluginsState::Adding {
                    bevy_tasks::tick_global_task_pools_on_main_thread();
                }
                app.finish();
                app.cleanup();
            }

            let mut tick = move |app: &mut bevy_app::App| {
                let start_time = std::time::Instant::now();
                let wait_time = std::time::Duration::from_millis(500);

                app.update();
                // @todo: find a better way to do this (there's `FixedUpdate`)
                // ^^^ however, i'm not sure how to alter the update frequency -> (change `Time<Fixed>` resource setting)
                // ^^^ plus, there's multiple different timers working simultaneously (archive + data scanning)
                // if archive_update_timer.just_finished() {
                //     app.world_mut().run_schedule(storage::DataArchivalSchedule);
                // }

                if let Some(exit) = app.should_exit() {
                    return Err(exit);
                }

                let end_time = std::time::Instant::now();
                let exec_time = end_time - start_time;
                archive_update_timer.tick(exec_time);

                if exec_time < wait_time {
                    return Ok(Some(wait_time - exec_time));
                }

                Ok(None)
            };

            loop {
                match tick(&mut app) {
                    Ok(Some(delay)) => std::thread::sleep(delay),
                    Ok(None) => continue,
                    Err(exit) => {
                        // always archive on exit to save state
                        // app.world_mut().run_schedule(storage::DataArchivalSchedule);
                        return exit
                    }
                }
            }
        });
    }
}

// provides facilities + setup for scripting environments
pub struct ScriptingPlugin;

impl bevy_app::Plugin for ScriptingPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_asset::<ScriptAsset>()
            .init_asset_loader::<ScriptAssetLoader>()
            .add_systems(bevy_app::Startup, files::renamer::load_renamer_script);
        // required constructs:
        // - script processing (see `bevy_mod_scripting`)
        // -- pretty much handling initial script setup and loading into the system
        // - script context/environment (per script)
        // >>> setup `mlua::Lua` and pass it in
        // - script context initialization
        // -- requires a different context depending on script type (e.g. renaming, data source, etc)
        // -- `bevy_mod_scripting` seems to use a hashmap connecting a script "context" to the scripts using it -> context is stored in "entries" along with load state
        // - script load handling
        // -- also handled by script execution
        // - script execution handling
        // -- also (may be) handled by script value extraction
        // - extracing values from scripts (i.e. `Chunk::call/exec`)
        //
        // ^^ rip from `bevy_mod_scripting`
        //
        // how can i load a script in one place, and access it in another?
        // ^^ i'm sure components or resources could handle this, but how? -> i need some sort of pipeline to handle execution
        // -- should scripts run *fully* on every execution? -> my hunch is yes (`bevy_mod_scripting` and (re)load and unload scripts with state, which is out of scope)
        //
        // renamer script
        // - as of now, only one is allowed (resource)
        //
        // data source script
        // @todo
        // - multiple allowed (can i check for one per data source?) -> can i register scripts with data sources?
    }
}
