use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

mod ir;
mod parser;
mod render;

#[derive(Debug, Parser)]
#[command(
    version,
    about = "A native Markdown-to-PDF renderer with no HTML/browser print step."
)]
struct Args {
    /// Markdown input file.
    input: PathBuf,

    /// PDF output file.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// TOML render configuration file. CLI flags override file values.
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Page size: letter or a4.
    #[arg(long)]
    page_size: Option<String>,

    /// Horizontal page margin in points.
    #[arg(long)]
    margin_x: Option<f32>,

    /// Top page margin in points.
    #[arg(long)]
    margin_top: Option<f32>,

    /// Bottom page margin in points.
    #[arg(long)]
    margin_bottom: Option<f32>,

    /// Body font size in points.
    #[arg(long)]
    body_size: Option<f32>,

    /// Body line height in points.
    #[arg(long)]
    body_line_height: Option<f32>,

    /// Table font size in points.
    #[arg(long)]
    table_size: Option<f32>,

    /// Table line height in points.
    #[arg(long)]
    table_line_height: Option<f32>,

    /// Code font size in points.
    #[arg(long)]
    code_size: Option<f32>,

    /// Code line height in points.
    #[arg(long)]
    code_line_height: Option<f32>,

    /// Syntect theme for fenced code blocks.
    #[arg(long)]
    code_theme: Option<String>,

    /// Disable syntax highlighting in fenced code blocks.
    #[arg(long)]
    no_code_highlighting: bool,

    /// Math backend: latex or fallback.
    #[arg(long)]
    math_mode: Option<String>,

    /// Disable page numbers in the footer.
    #[arg(long)]
    no_page_numbers: bool,

    /// Maximum rendered image height in points.
    #[arg(long)]
    max_image_height: Option<f32>,

    /// Disable image captions from Markdown alt text.
    #[arg(long)]
    no_image_captions: bool,

    /// Image caption font size in points.
    #[arg(long)]
    image_caption_size: Option<f32>,

    /// Gap between image and caption in points.
    #[arg(long)]
    image_caption_gap: Option<f32>,

    /// PDF document title.
    #[arg(long)]
    title: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let input = args.input.clone();
    let output = args
        .output
        .clone()
        .unwrap_or_else(|| input.with_extension("pdf"));
    let source = std::fs::read_to_string(&input)
        .with_context(|| format!("failed to read {}", input.display()))?;
    let document = parser::parse_markdown(&source);
    let base_dir = input.parent().unwrap_or_else(|| std::path::Path::new("."));
    let options = render_options(&args)?;

    render::render_pdf_with_options(&document, &output, base_dir, options)?;
    println!("Rendered {}", output.display());
    Ok(())
}

fn render_options(args: &Args) -> Result<render::RenderOptions> {
    let mut options = if let Some(config) = &args.config {
        let text = std::fs::read_to_string(config)
            .with_context(|| format!("failed to read config {}", config.display()))?;
        toml::from_str::<render::RenderOptions>(&text)
            .with_context(|| format!("failed to parse config {}", config.display()))?
    } else {
        render::RenderOptions::default()
    };

    if let Some(page_size) = &args.page_size {
        match page_size.to_ascii_lowercase().as_str() {
            "letter" => {
                options.page_width_pt = 612.0;
                options.page_height_pt = 792.0;
            }
            "a4" => {
                options.page_width_pt = 595.28;
                options.page_height_pt = 841.89;
            }
            value => anyhow::bail!("unsupported page size `{value}`; use `letter` or `a4`"),
        }
    }

    if let Some(value) = args.margin_x {
        options.margin_x_pt = value;
    }
    if let Some(value) = args.margin_top {
        options.margin_top_pt = value;
    }
    if let Some(value) = args.margin_bottom {
        options.margin_bottom_pt = value;
    }
    if let Some(value) = args.body_size {
        options.body_size_pt = value;
    }
    if let Some(value) = args.body_line_height {
        options.body_line_height_pt = value;
    }
    if let Some(value) = args.table_size {
        options.table_size_pt = value;
    }
    if let Some(value) = args.table_line_height {
        options.table_line_height_pt = value;
    }
    if let Some(value) = args.code_size {
        options.code_size_pt = value;
    }
    if let Some(value) = args.code_line_height {
        options.code_line_height_pt = value;
    }
    if let Some(value) = &args.code_theme {
        options.code_theme = value.clone();
    }
    if args.no_code_highlighting {
        options.code_highlighting = false;
    }
    if let Some(math_mode) = &args.math_mode {
        options.math_mode = match math_mode.to_ascii_lowercase().as_str() {
            "latex" => render::MathMode::Latex,
            "fallback" => render::MathMode::Fallback,
            value => anyhow::bail!("unsupported math mode `{value}`; use `latex` or `fallback`"),
        };
    }
    if args.no_page_numbers {
        options.page_numbers = false;
    }
    if let Some(value) = args.max_image_height {
        options.max_image_height_pt = value;
    }
    if args.no_image_captions {
        options.image_captions = false;
    }
    if let Some(value) = args.image_caption_size {
        options.image_caption_size_pt = value;
    }
    if let Some(value) = args.image_caption_gap {
        options.image_caption_gap_pt = value;
    }
    if let Some(value) = &args.title {
        options.title = value.clone();
    }

    Ok(options)
}
