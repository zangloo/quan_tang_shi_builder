mod loader;

use std::env::temp_dir;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::Read;
use anyhow::{anyhow, Result};
use clap::Parser;
use epub_builder::EpubBuilder;
use epub_builder::ZipLibrary;
use epub_builder::EpubContent;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use handlebars::Handlebars;
use rand::distributions::Alphanumeric;
use rand::Rng;
use crate::loader::{Chapter, load_data, Metadata};

pub const TOC_RENDER: &str = "toc";
pub const TEXT_RENDER: &str = "text";

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
	#[clap(short, long, help = "Source repository", default_value = "https://github.com/chinese-poetry/chinese-poetry")]
	source: String,
	#[clap(short, long, help = "Pre cloned Source repository, if set, ignore --source")]
	cloned_source: Option<String>,
	#[clap(short, long, help = "Overwrite if output file exists")]
	overwrite: bool,
	#[clap(short, long, help = "Text template file path")]
	text: Option<PathBuf>,
	#[clap(short = 'y', long, help = "Css file path")]
	css: Option<PathBuf>,
	output: PathBuf,
}

fn main() -> Result<()>
{
	let cli = Cli::parse();
	if !cli.overwrite && cli.output.exists() {
		return Err(anyhow!("output path exists: {:#?}", cli.output));
	}
	let (repo_path, tmp) = fetch(&cli.source, &cli.cloned_source)?;
	let result = build(&repo_path, &cli.output, cli.overwrite, &cli.text, &cli.css);
	if tmp {
		fs::remove_dir_all(&repo_path)?;
	}
	result
}

fn fetch(source: &str, cloned_source: &Option<String>) -> Result<(PathBuf, bool)>
{
	if let Some(cloned_source) = cloned_source {
		let repo_path = PathBuf::from_str(&cloned_source)?;
		return if repo_path.exists() {
			if repo_path.is_dir() {
				Ok((repo_path, false))
			} else {
				Err(anyhow!("cloned_source is not a dir: {}", cloned_source))
			}
		} else {
			Err(anyhow!("cloned_source not exists: {}", cloned_source))
		};
	}
	let mut tmp = temp_dir();
	let mut s: String = rand::thread_rng()
		.sample_iter(&Alphanumeric)
		.take(8)
		.map(char::from)
		.collect();
	s.insert_str(0, "shici_");
	tmp.push(s);
	let tmp_path = tmp.to_str().expect("failed get temp dir").to_string();
	if let Err(e) = Command::new("git")
		.args([
			"clone",
			"--depth",
			"1",
			source,
			&tmp_path,
		])
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.output() {
		return Err(anyhow!("failed to clone repository {} with error: {}", source, e.to_string()));
	}
	Ok((tmp, true))
}

fn build(repo_path: &PathBuf, output: &PathBuf, overwrite: bool, text: &Option<PathBuf>, css: &Option<PathBuf>) -> Result<()>
{
	let mut handlebars = Handlebars::new();
	let text_template = if let Some(text_template_path) = text {
		println!("using custom text template: {:#?}", text_template_path);
		fs::read_to_string(text_template_path)?
	} else {
		include_str!("../asset/text.hbs").to_string()
	};
	handlebars.register_template_string(TEXT_RENDER, text_template)?;
	handlebars.register_template_string(TOC_RENDER, include_str!("..//asset/toc.hbs"))?;
	let (metadata, chapters) = load_data(repo_path, &handlebars)?;
	let file = if overwrite {
		OpenOptions::new().create(true).truncate(true).write(true).open(output)?
	} else {
		OpenOptions::new().create_new(true).write(true).open(output)?
	};

	println!("building epub...");
	let result = if let Some(css) = css {
		println!("using custom stylesheet: {:#?}", css);
		let vec = fs::read(css)?;
		let css = vec.as_slice();
		build_epub(metadata, chapters, file, css)
	} else {
		let css = include_bytes!("../asset/style.css").as_slice();
		build_epub(metadata, chapters, file, css)
	};
	if let Err(e) = result {
		if output.exists() {
			fs::remove_file(output)?;
		}
		Err(anyhow!("failed build epub: {}", e.to_string()))
	} else {
		println!("epub success saved to {:#?}", output);
		Ok(())
	}
}

fn build_epub<R: Read>(metadata: Metadata, chapters: Vec<Chapter>, file: File, css: R) -> epub_builder::Result<()>
{
	let mut epub = EpubBuilder::new(ZipLibrary::new()?)?;
	let epub = epub
		// Set some metadata
		.metadata("author", metadata.author)?
		.metadata("title", metadata.title)?
		.stylesheet(css)?;

	for chapter in chapters {
		epub.add_content(EpubContent::new(chapter.filename, chapter.content.as_bytes())
			.title(chapter.title)
			.reftype(chapter.reference_type))?;
	}
	// Finally, write the EPUB file to stdout
	epub.generate(&file)
}