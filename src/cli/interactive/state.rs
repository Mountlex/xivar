use anyhow::Result;
use itertools::Itertools;
use std::{fmt::Display, io::Write};
use termion::{clear, color, cursor, event::Key};

use crate::{merge_to_papers, Paper, PaperHit};

use super::Action;

#[derive(Clone)]
pub struct StateData {
    term: String,
    papers: Vec<Paper>,
    state: State,
}

impl StateData {
    pub fn new() -> Self {
        Self {
            term: String::new(),
            papers: vec![],
            state: State::Idle,
        }
    }

    pub fn papers(&self) -> &Vec<Paper> {
        &self.papers
    }

    pub fn clear_papers(&mut self) {
        self.papers.clear()
    }

    pub fn merge_to_papers(&mut self, hits: Vec<PaperHit>) {
        merge_to_papers(&mut self.papers, hits.into_iter());
    }

    pub fn term(&self) -> &str {
        &self.term
    }

    pub fn to_idle(&mut self) {
        self.state = State::Idle
    }

    pub fn state_transition(&mut self, key: Key) -> Option<Action> {
        let current_state = self.state.clone();
        match (key, current_state) {
            (Key::Ctrl('c'), _) => Some(Action::Quit),
            (Key::Char(c), State::Idle) => {
                self.term.push(c);
                self.state = State::Searching;
                Some(Action::UpdateSearch)
            }
            (Key::Char(c), State::Searching) => {
                self.term.push(c);
                Some(Action::UpdateSearch)
            }
            (Key::Backspace, State::Searching | State::Idle) => {
                self.term.pop();
                if self.term.is_empty() {
                    self.state = State::Idle;
                    self.papers.clear();
                } else {
                    self.state = State::Searching;
                }
                Some(Action::UpdateSearch)
            }
            (Key::Down, State::Idle) => {
                if self.papers.len() > 0 {
                    self.state = State::Scrolling(0);
                    Some(Action::Reprint)
                } else {
                    None
                }
            }
            (Key::Down, State::Scrolling(i)) => {
                if i < self.papers.len() as u16 - 1 {
                    self.state = State::Scrolling(i + 1);
                }
                Some(Action::Reprint)
            }
            (Key::Up, State::Scrolling(i)) => {
                if i > 0 {
                    self.state = State::Scrolling(i - 1);
                } else {
                    self.state = State::Idle;
                }
                Some(Action::Reprint)
            }
            (Key::Char('s'), State::Scrolling(_)) => {
                self.state = State::Idle;
                Some(Action::Reprint)
            }
            (Key::Char('\n'), State::Scrolling(i)) => {
                let selected: &Paper = &self.papers[i as usize];
                let hit = selected.0.first().unwrap();
                match hit {
                    PaperHit::Local(paper) => open::that(&paper.location).ok()?,
                    PaperHit::Dblp(ref paper) => open::that(paper.ee.raw()).ok()?,
                    PaperHit::Arxiv(ref paper) => open::that(paper.ee.raw()).ok()?,
                }
                None
            }
            (Key::Char(s), State::Scrolling(i)) => {
                if s.is_numeric() {
                    let j = s.to_digit(10).unwrap();
                    let selected: &Paper = &self.papers[i as usize];
                    let selected_hit = &selected.0[(j - 1) as usize];
                    self.state = State::SelectedHit {
                        index: i,
                        hit: selected_hit.clone(),
                    };
                    Some(Action::Reprint)
                } else {
                    None
                }
            }
            (Key::Char(s), State::SelectedHit { index: _, hit }) => {
                match &hit {
                    PaperHit::Local(paper) => open::that(&paper.location).ok()?,
                    PaperHit::Dblp(paper) => {
                        if s == '1' {
                            open::that(paper.ee.raw()).ok()?
                        }
                        if s == '2' {
                            open::that(paper.url.raw()).ok()?
                        }
                        if s == '3' {
                            return Some(Action::FetchToClip(paper.bib_url()));
                        }
                    }
                    PaperHit::Arxiv(paper) => {
                        if s == '1' {
                            return Some(Action::Download(
                                paper.metadata().clone(),
                                paper.download_url(),
                            ));
                        }
                        if s == '2' {
                            open::that(paper.ee.raw()).ok()?
                        }
                    }
                }
                Some(Action::Reprint)
            }
            (Key::Esc, State::SelectedHit { index: i, hit: _ }) => {
                self.state = State::Scrolling(i);
                Some(Action::Reprint)
            }
            (Key::Esc, State::Scrolling(_)) => {
                self.state = State::Searching;
                Some(Action::Reprint)
            }
            _ => None,
        }
    }

    pub fn write_to_terminal<W: std::io::Write>(&self, writer: &mut W) -> Result<()> {
        write!(
            writer,
            "{hide}{goto}{clear}",
            hide = cursor::Hide,
            goto = cursor::Goto(1, 4),
            clear = clear::AfterCursor
        )
        .ok();
        let (_width, height) = termion::terminal_size().unwrap_or((80, 20));

        write!(
            writer,
            "{hide}{goto}{clear}Search: {}",
            self.term,
            hide = cursor::Hide,
            goto = cursor::Goto(1, 1),
            clear = clear::CurrentLine
        )
        .ok();

        match &self.state {
            State::Searching => info(&String::from("Searching...")),
            State::Scrolling(i) => {
                let selected: &Paper = &self.papers[*i as usize];
                let string: String = selected
                    .0
                    .iter()
                    .enumerate()
                    .map(|(i, hit)| format!("({}) {}", i + 1, hit.remote_tag()))
                    .join("  ");
                info(&format!("Select remote: {}", string))
            }
            State::Idle => {
                if self.papers().len() > 0 {
                    info(&format!("Found {} results!", self.papers().len()));
                } else {
                    info(&String::from(""));
                }
            }
            State::SelectedHit { index: _, hit } => match hit {
                PaperHit::Local(paper) => {
                    info(&format!("Select action: (1) open {:?}", paper.location))
                }
                PaperHit::Dblp(paper) => info(&format!(
                    "Select action: (1) {:15}  (2) {:15}  (3) Show bib file",
                    paper.ee.raw(),
                    paper.url.raw()
                )),
                PaperHit::Arxiv(_) => {
                    info(&format!("Select action: (1) Download  (2) open online"))
                }
            },
        }

        for (i, paper) in self.papers().iter().enumerate().take((height - 5) as usize) {
            if self.state == State::Scrolling(i as u16) {
                write!(
                    writer,
                    "{hide}{goto}{clear}{color}{}",
                    paper,
                    hide = cursor::Hide,
                    goto = cursor::Goto(1, i as u16 + 4),
                    clear = clear::CurrentLine,
                    color = color::Bg(color::Blue),
                )?;
            } else {
                write!(
                    writer,
                    "{hide}{goto}{clear}{}",
                    paper,
                    hide = cursor::Hide,
                    goto = cursor::Goto(1, i as u16 + 4),
                    clear = clear::CurrentLine,
                )?;
            }
        }
        writer.flush()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Idle,
    Searching,
    Scrolling(u16),
    SelectedHit { index: u16, hit: PaperHit },
}

fn info<I: Display>(item: &I) {
    let buf = format!("{}", item);
    write!(
        std::io::stdout(),
        "{hide}{goto}{clear}{}",
        buf,
        hide = cursor::Hide,
        goto = cursor::Goto(1, 2),
        clear = clear::CurrentLine
    )
    .ok();
    std::io::stdout().flush().ok();
}
