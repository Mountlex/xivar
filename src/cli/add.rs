use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};
use clap::Clap;
use dialoguer::Input;
use lopdf::{Document, Object};

use super::Command;

#[derive(Clap, Debug)]
pub struct Add {
    #[clap(parse(from_os_str))]
    pdf_file: PathBuf,
}

impl Command for Add {
    fn run(&self) -> Result<()> {
        if "pdf" == self.pdf_file.extension().unwrap() {
            let doc = Document::load(&self.pdf_file)?;
            let title = if let Ok(title) = get_title(&doc) {
                println!("Title: {}", title);
                title
            } else {
                Input::new().with_prompt("Title").interact_text()?
            };
            let authors = if let Ok(author) = get_author(&doc) {
                println!("Authors: {}", author);
                author
            } else {
                Input::new().with_prompt("Authors").interact_text()?
            };
            let year: String = Input::new().with_prompt("Year").interact_text()?;
        } else {
            bail!("Cannot read file!");
        }

        Ok(())
    }
}

fn get_title(doc: &Document) -> Result<String> {
    get_info_field(doc, "Title")
}

fn get_author(doc: &Document) -> Result<String> {
    get_info_field(doc, "Author")
}

fn get_info_field(doc: &Document, field_name: &str) -> Result<String> {
    let info = doc.trailer.get(b"Info").map_err(|err| anyhow!(err))?;
    if let Some(dict) = match *info {
        Object::Dictionary(ref dict) => Some(dict),
        Object::Reference(ref id) => doc.objects.get(id).and_then(|o| o.as_dict().ok()),
        _ => None,
    } {
        let entry = dict
            .get(field_name.as_bytes())
            .map(|obj| {
                let field = std::str::from_utf8(obj.as_str().unwrap())
                    .unwrap()
                    .to_owned();
                field.replace("(", "").replace(")", "")
            })
            .map_err(|err| anyhow::anyhow!(err));
        if let Ok(ref field) = entry {
            if field.is_empty() {
                Err(anyhow!("Field empty!"))
            } else {
                entry
            }
        } else {
            entry
        }
    } else {
        Err(anyhow!("Could not read {} from pdf file!", field_name))
    }
}
