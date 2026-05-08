use std::io::{self, Stdout};
use std::rc::Rc;
use std::sync::Arc;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::Terminal;

use super::{Context, Painter, TabBar, View};
use crate::app::Config;
use crate::Result;

#[allow(clippy::redundant_closure)]
pub fn init(
    _config: Arc<Config>,
    views: Vec<View>,
) -> Result<Interface<CrosstermBackend<Stdout>>> {
    debug_assert!(!views.is_empty());

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    let tab_titles = views.iter().map(|view| view.title()).collect::<Vec<_>>();
    let tabs = TabBar::new(tab_titles);

    Ok(Interface {
        terminal,
        views,
        tabs,
    })
}

/// Interface is a group tabs and tab contents
#[derive(Debug)]
pub struct Interface<B: Backend> {
    terminal: Terminal<B>,
    views: Vec<View>,
    tabs: TabBar,
}

impl<B: Backend> Interface<B>
where
    B::Error: Into<io::Error>,
{
    pub fn draw(&mut self) -> Result<()> {
        let context = Rc::new(Context {
            tabs: &self.tabs,
            view: &self.views[self.tabs.index()],
        });
        self.terminal
            .draw(|frame| {
                Painter::from_context(context.clone()).draw(frame);
            })
            .map_err(|e| crate::Error::Io(e.into()))?;

        Ok(())
    }

    pub fn views_mut(&mut self) -> &mut [View] {
        self.views.as_mut()
    }

    pub fn tabs_mut(&mut self) -> &mut TabBar {
        &mut self.tabs
    }
}

impl<B: Backend> Drop for Interface<B> {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        execute!(stdout, LeaveAlternateScreen, DisableMouseCapture).unwrap();
        disable_raw_mode().unwrap();
    }
}
