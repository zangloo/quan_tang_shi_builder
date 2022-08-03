use std::fs;
use std::fs::OpenOptions;
use anyhow::Result;
use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;
use epub_builder::ReferenceType;
use handlebars::Handlebars;
use crate::{TEXT_RENDER, TOC_RENDER};

pub struct Metadata {
	pub author: String,
	pub title: String,
}

pub struct Chapter {
	pub reference_type: ReferenceType,
	pub title: String,
	pub filename: String,
	pub content: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Entry {
	title: String,
	author: String,
	// 传
	biography: String,
	paragraphs: Vec<String>,
	notes: Vec<String>,
	volume: String,
	#[serde(rename = "no#")]
	seq: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TocEntry {
	filename: String,
	title: String,
}

pub fn load_data(repo_path: &PathBuf, handlebars: &Handlebars) -> Result<(Metadata, Vec<Chapter>)>
{
	let metadata = Metadata {
		author: "彭定求、沈三曾、汪士纮、汪繹、俞梅、楊中訥、徐樹本、車鼎晉、潘從律、查嗣瑮".to_string(),
		title: "全唐詩".to_string(),
	};
	let mut chapters = vec![];
	let source_dir = repo_path.join("quan_tang_shi/json");
	let mut names = vec![];
	for entry in fs::read_dir(&source_dir)? {
		let entry = entry?;
		let filename = entry.file_name();
		if let Some(filename) = filename.to_str() {
			if filename.ends_with(".json") {
				names.push(filename[..filename.len() - 5].to_string());
			}
		}
	}
	names.sort();
	for name in &names {
		if let Some(chapter) = load_chapter(&source_dir, name, handlebars)? {
			chapters.push(chapter);
		}
	}
	let mut tocs = vec![];
	for chapter in &chapters {
		tocs.push(TocEntry {
			filename: chapter.filename.clone(),
			title: chapter.title.clone(),
		});
	}

	println!("creating TOC");
	let toc_content = handlebars.render(TOC_RENDER, &tocs)?;
	let toc_chapter = Chapter {
		reference_type: ReferenceType::Toc,
		title: "目録".to_string(),
		filename: "toc.xhtml".to_string(),
		content: toc_content,
	};
	chapters.insert(0, toc_chapter);
	Ok((metadata, chapters))
}

fn load_chapter(source_dir: &PathBuf, name: &str, handlebars: &Handlebars) -> Result<Option<Chapter>>
{
	let mut source_name = name.to_string();
	source_name.push_str(".json");
	let path = source_dir.join(&source_name);
	println!("processing {:#?}", path);
	let file = OpenOptions::new().read(true).open(path)?;
	let entries: Vec<Entry> = serde_json::from_reader(file)?;
	if entries.len() == 0 {
		return Ok(None);
	}
	let title = entries.get(0).unwrap().volume.clone();
	let content = handlebars.render(TEXT_RENDER, &entries)?;
	let mut filename = name.to_string();
	filename.push_str(".xhtml");
	Ok(Some(Chapter {
		reference_type: ReferenceType::Text,
		title,
		filename,
		content,
	}))
}
