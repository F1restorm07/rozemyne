use backend_types::lua::ScriptAsset;
use bevy_reflect::TypePath;

#[derive(Default, TypePath)]
pub struct ScriptAssetLoader;

impl bevy_asset::AssetLoader for ScriptAssetLoader {
    type Asset = ScriptAsset;
    type Settings = ();
    type Error = String;
    async fn load(
        &self,
        reader: &mut dyn bevy_asset::io::Reader,
        settings: &Self::Settings,
        load_context: &mut bevy_asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut content = Vec::new();
        reader.read_to_end(&mut content).await.unwrap();
        Ok(ScriptAsset { content: content.into_boxed_slice() })
    }
    fn extensions(&self) -> &[&str] {
        &["lua"]
    }
}


