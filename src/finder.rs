use anyhow::{anyhow, bail, Result};
use skim::prelude::*;

use crate::remotes::{local::LocalPaper, Paper};

impl SkimItem for Paper {
    fn text(&self) -> Cow<str> {
        Cow::Owned(format!("{}", self))
    }

    fn display<'a>(&'a self, _context: DisplayContext<'a>) -> AnsiString<'a> {
        AnsiString::parse(&format!("{}", self))
    }
}

impl SkimItem for LocalPaper {
    fn text(&self) -> Cow<str> {
        Cow::Owned(format!("{}", self))
    }

    fn display<'a>(&'a self, _context: DisplayContext<'a>) -> AnsiString<'a> {
        AnsiString::parse(&format!("{}", self))
    }
}

pub fn show_and_select<I, T>(iter: T) -> Result<I>
where
    T: Iterator<Item = I>,
    I: SkimItem + Clone,
{
    let options = SkimOptionsBuilder::default()
        .height(Some("100%"))
        .build()
        .unwrap();

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();
    for item in iter {
        let _ = tx_item.send(Arc::new(item));
    }

    drop(tx_item); // so that skim could know when to stop waiting for more items.

    if let Some(output) = Skim::run_with(&options, Some(rx_item)) {
        if !output.is_abort {
            output
                .selected_items
                .into_iter()
                .nth(0)
                .map(move |item| {
                    (*item)
                        .as_any()
                        .downcast_ref::<I>() // downcast to concrete type
                        .expect("something wrong with downcast")
                        .clone()
                })
                .ok_or(anyhow!("Internal error"))
        } else {
            bail!("No entry selected! Aborting...")
        }
    } else {
        bail!("Internal error")
    }
}
