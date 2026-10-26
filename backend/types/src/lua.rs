use bevy_asset::{Asset, Handle};

pub struct LuaContext(pub mlua::Lua);

#[derive(Clone, Asset, bevy_reflect::Reflect)]
#[reflect(opaque)]
pub struct ScriptAsset {
    pub content: Box<[u8]>,
}
