use clap::Parser;
use showme::{BackendKind, Cli, Config, Renderer};

mod common;
use common::TestDirectory;

#[test]
fn library_defaults_render_the_same_image_as_cli_defaults() {
    let directory = TestDirectory::new();
    let input = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/red.ppm");
    let library_output = directory.path().join("library.txt");
    let cli_output = directory.path().join("cli.txt");
    let config = Config {
        inputs: vec![input.clone()],
        backend: BackendKind::Unicode,
        quiet: true,
        output_file: Some(library_output.clone()),
        ..Config::default()
    };
    Renderer::build(config).unwrap().run().unwrap();
    let config = Cli::try_parse_from([
        "showme",
        "--quiet",
        "--backend",
        "unicode",
        "-o",
        cli_output.to_str().unwrap(),
        input.to_str().unwrap(),
    ])
    .unwrap()
    .into_config()
    .unwrap();
    Renderer::build(config).unwrap().run().unwrap();
    let library = std::fs::read(library_output).unwrap();
    assert!(!library.is_empty());
    assert_eq!(library, std::fs::read(cli_output).unwrap());
}
