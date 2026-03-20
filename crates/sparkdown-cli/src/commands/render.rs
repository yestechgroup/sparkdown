use anyhow::{bail, Context, Result};
use sparkdown_core::parser::SparkdownParser;
use sparkdown_render::html_rdfa::HtmlRdfaRenderer;
use sparkdown_render::jsonld::JsonLdRenderer;
use sparkdown_render::traits::OutputRenderer;
use sparkdown_render::turtle::TurtleRenderer;
use std::fs;
use std::io;

pub fn run(input: &str, format: &str, output: Option<&str>) -> Result<()> {
    let source = if input == "-" {
        io::read_to_string(io::stdin())?
    } else {
        fs::read_to_string(input).with_context(|| format!("Failed to read {input}"))?
    };

    let parser = SparkdownParser::new();
    let doc = parser
        .parse(&source)
        .with_context(|| format!("Failed to parse {input}"))?;

    let renderer: Box<dyn OutputRenderer> = match format {
        "html" => Box::new(HtmlRdfaRenderer::new()),
        "jsonld" | "json-ld" => Box::new(JsonLdRenderer::new()),
        "turtle" | "ttl" => Box::new(TurtleRenderer::new()),
        other => bail!("Unknown format: {other}. Supported: html, jsonld, turtle"),
    };

    if let Some(path) = output {
        let mut file =
            fs::File::create(path).with_context(|| format!("Failed to create {path}"))?;
        renderer
            .render(&doc, &mut file)
            .with_context(|| "Render failed")?;
        eprintln!("Written to {path}");
    } else {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        renderer
            .render(&doc, &mut handle)
            .with_context(|| "Render failed")?;
    }

    Ok(())
}
