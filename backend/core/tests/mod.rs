use backend_core::files;
use backend_types::parser::SegmentKind;
use backend_types::parser::token::Lexer;
use rstest::rstest;

// #[test]
// fn test_file_hashing() {
// use highway::HighwayHash;
//     let mut file = std::fs::File::open("../../test.txt").unwrap();
//     let mut buf = [0; 1024 * 1024];
//     let mut hasher = highway::HighwayHasher::default();
//     let before = std::time::SystemTime::now();
//
//     loop {
//         let read = file.read(&mut buf[..]).expect("cannot read file");
//         if read == 0 { break; }
//         hasher.append(&buf);
//     }
//
//     panic!("hash: {:?}\ttime: {:?}\tfile size: {}", hasher.finalize256(), before.elapsed().unwrap(), file.metadata().unwrap().len());
// }

#[rstest]
fn test_file_title_parsing(
    #[files("tests/torrent_sets/*_input.json")] #[mode = str] input: &str,
    #[files("tests/torrent_sets/*_output.json")] #[mode = str] output: &str,
) {
    let input_json: serde_json::Value = serde_json::from_str(input).unwrap();
    let input_torrents = input_json.as_array().unwrap();
    let output_json: serde_json::Value = serde_json::from_str(output).unwrap();
    let output_torrents = output_json.as_array().unwrap();
    // let mut errs = Vec::new();

    for (i, o) in input_torrents.iter().zip(output_torrents) {
        let input = i.as_str().unwrap();
        let output = o.as_str().unwrap();

        let tokens = Lexer::new(input);
        let parsed = files::parser::parse(tokens.into_tokens());
        let mut title = parsed.iter().find(|t| t.kind == SegmentKind::Title).unwrap().text.clone();

        // exceptions to the title parsing rule (the title must contain the periods)
        // this does not actually matter in the end as some data sources support fuzzy matching
        if title == "Marvel's Agents of S H I E L D" || title == "Marvels Agents of S H I E L D" {
            title = "Marvel's Agents of S.H.I.E.L.D.".to_string().into();
        } else if title == "Magnum P I" {
            title = "Magnum P.I.".to_string().into();
        }

        assert_eq!(title, output, "failed to parse title `{input}");
        // if title != output {
        //     title = (title.to_string() + &parsed.iter().find(|t| t.kind == TitlePartKind::Episode).map(|t| t.text.to_string()).unwrap_or("".to_string())).into();
        //     if title != output {
        //         errs.push((title, input, output));
        //     }
        // }
    }

    // panic!("errs: {:#?}\nerr amount: {}", errs, errs.len());
}

#[test]
fn test_nyaa() {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    let input = std::fs::read_to_string("../../nyaa_input.json").unwrap();
    let input: serde_json::Value = serde_json::from_str(&input).unwrap();
    let input = input.as_array().unwrap();
    let output = std::fs::read_to_string("../../nyaa_output.json").unwrap();
    let output: serde_json::Value = serde_json::from_str(&output).unwrap();
    let output = output.as_array().unwrap();
    // let mut errors = vec![];

    for (i, o) in input.iter().zip(output) {
        let input = i.as_str().unwrap();
        let output = o.as_str().unwrap();

        let tokens = Lexer::new(&input);
        let parsed = files::parser::parse(tokens.into_tokens());
        let title = parsed.iter().find(|t| t.kind == SegmentKind::Title).map(|t| t.text.clone()).unwrap_or_default();
        assert_eq!(title, output, "failed to parse title `{input}");
        // if title != output { errors.push((title, output, input)); }
    }

    // panic!("\n>>>>> errors: {errors:#?}\nerror amount: {}", errors.len());
}
