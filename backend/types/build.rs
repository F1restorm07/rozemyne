fn main() {
    cynic_codegen::register_schema("anilist")
        .from_sdl_file("../graphql_schemas/anilist.graphql")
        .unwrap();
}
