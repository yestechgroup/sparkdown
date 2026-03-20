use anyhow::{Context, Result};
use std::fs;

pub fn run(output: &str, doc_type: &str) -> Result<()> {
    let template = match doc_type {
        "article" => ARTICLE_TEMPLATE,
        "event" => EVENT_TEMPLATE,
        "review" => REVIEW_TEMPLATE,
        "person" => PERSON_TEMPLATE,
        _ => {
            eprintln!("Unknown doc-type '{doc_type}', using article template.");
            eprintln!("Supported types: article, event, review, person");
            ARTICLE_TEMPLATE
        }
    };

    fs::write(output, template).with_context(|| format!("Failed to write {output}"))?;
    println!("Created {output} with {doc_type} template");

    Ok(())
}

const ARTICLE_TEMPLATE: &str = r#"---
title: My Article
"@type": schema:Article
prefixes:
  schema: http://schema.org/
---

# My Article {.schema:Article}

Your content here.
"#;

const EVENT_TEMPLATE: &str = r#"---
title: My Event
"@type": schema:Event
prefixes:
  schema: http://schema.org/
---

# My Event {.schema:Event startDate=2026-01-01 endDate=2026-01-02}

Event description here.
"#;

const REVIEW_TEMPLATE: &str = r#"---
title: My Review
"@type": schema:Review
prefixes:
  schema: http://schema.org/
---

# My Review {.schema:Review}

::: schema:Review
Your review content here.
:::
"#;

const PERSON_TEMPLATE: &str = r#"---
title: About Me
"@type": schema:Person
prefixes:
  schema: http://schema.org/
---

# About Me {.schema:Person}

:entity[Your Name]{type=schema:Person}
"#;
