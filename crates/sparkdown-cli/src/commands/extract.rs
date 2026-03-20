use anyhow::{bail, Context, Result};
use sparkdown_core::parser::SparkdownParser;
use sparkdown_render::jsonld::JsonLdRenderer;
use sparkdown_render::traits::OutputRenderer;
use sparkdown_render::turtle::TurtleRenderer;
use std::fs;
use std::io;

pub fn run(input: &str, format: &str) -> Result<()> {
    let source = fs::read_to_string(input).with_context(|| format!("Failed to read {input}"))?;

    let parser = SparkdownParser::new();
    let doc = parser
        .parse(&source)
        .with_context(|| format!("Failed to parse {input}"))?;

    let renderer: Box<dyn OutputRenderer> = match format {
        "turtle" | "ttl" => Box::new(TurtleRenderer::new()),
        "jsonld" | "json-ld" => Box::new(JsonLdRenderer::new()),
        other => bail!("Unknown format: {other}. Supported: turtle, jsonld"),
    };

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    renderer
        .render(&doc, &mut handle)
        .with_context(|| "Extract failed")?;

    Ok(())
}
