use bevy::prelude::*;

mod water;
use water::WaterPlugin;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WaterPlugin)
        .run()
}
