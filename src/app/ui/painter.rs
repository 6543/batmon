/// Layout schema:
///
/// ```
///           +------+------+--------------------------------------------------------+
/// Tabs   →  | BAT0 | BAT1 | BAT2                                                   |
///           +------+------+----------+---------------------------------------------+  <------\
/// SoC    →  |:::::::::: 65%          | Voltage graph                               |         |
///           +------------------------|                                             |         |
/// Common    |                        | 33 % of the right column                    |         |
/// info   →  | Vendor: …              |                                             |         |
///           | Model: …               |                                             |
///           | S/N: …                 |                                             |
///           +------------------------+---------------------------------------------+         m
/// Energy    | Voltage: …             | Consumption graph                           |         a
/// info   →  | Consumption: …         |                                             |         i
///           +------------------------+ 33 % of the right column                    |         n
/// Timings   | Time to full: …        |                                             |
///        →  | Time to empty: …       |                                             |         w
///           +------------------------+                                             |         i
/// Environ   | Temperature: …         |                                             |         n
///        →  |                        +---------------------------------------------+         d
///           |                        | Temperature graph                           |         o
///           |                        |                                             |         w
///           |                        | 33+1 % of the right column                  |
///           |                        |                                             |         |
///           |                        |                                             |         |
///           |                        |                                             |         |
///           +------------------------+---------------------------------------------+         /
///                                                                                           /
///           ^            ↑                       ↑                                         /
///           |       left column            right column                                   /
///           \                                                                            /
///            \------------------ main window -------------------------------------------/
/// ```
use std::ops::Deref;
use std::rc::Rc;
use std::time::Duration;

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Axis, Block, Borders, Chart, Dataset, Gauge, Paragraph, Row, Table, Tabs};
use ratatui::Frame;

use starship_battery::units::electric_potential::volt;
use starship_battery::units::energy::{joule, watt_hour};
use starship_battery::units::power::watt;
use starship_battery::units::ratio::{percent, ratio};
use starship_battery::units::thermodynamic_temperature::{degree_celsius, kelvin};
use starship_battery::units::time::second;
use starship_battery::units::Unit;
use starship_battery::State;

use super::{ChartData, TabBar, Units, View};

#[derive(Debug)]
pub struct Context<'i> {
    pub tabs: &'i TabBar,
    pub view: &'i View,
}

#[derive(Debug)]
pub struct Painter<'i>(Rc<Context<'i>>);

impl<'i> Painter<'i> {
    pub fn from_context(context: Rc<Context<'i>>) -> Painter<'i> {
        Painter(context)
    }

    pub fn draw(&self, frame: &mut Frame) {
        let main = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3), // Tabs
                    Constraint::Min(10),   // Main window
                ]
                .as_ref(),
            )
            .split(frame.area());

        // Left column with info and right column with graphs
        let main_columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Length(40), // Information
                    Constraint::Min(20),    // Graphs
                ]
                .as_ref(),
            )
            .split(main[1]);

        // Percentage bar and information table
        let left_column = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3),  // percentage bar
                    Constraint::Length(10), // common info
                    Constraint::Length(9),  // energy stuff
                    Constraint::Length(5),  // timings
                    Constraint::Min(4),     // environment
                ]
                .as_ref(),
            )
            .split(main_columns[0]);

        // Graphs
        let right_column = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage(33), // Voltage
                    Constraint::Percentage(33), // Consumption
                    Constraint::Percentage(34), // Temperature
                ]
                .as_ref(),
            )
            .split(main_columns[1]);

        // Drawing all the things now!
        self.draw_tabs(frame, main[0]);
        self.draw_state_of_charge_bar(frame, left_column[0]);
        self.draw_common_info(frame, left_column[1]);
        self.draw_energy_info(frame, left_column[2]);
        self.draw_timing_info(frame, left_column[3]);
        self.draw_environment_info(frame, left_column[4]);
        self.draw_chart(self.view.voltage(), frame, right_column[0]);
        self.draw_chart(self.view.energy_rate(), frame, right_column[1]);
        self.draw_chart(self.view.temperature(), frame, right_column[2]);
    }

    pub fn draw_tabs(&self, frame: &mut Frame, area: Rect) {
        let titles: Vec<Line> = self
            .tabs
            .titles()
            .iter()
            .map(|t| Line::from(t.as_str()))
            .collect();
        let tabs = Tabs::new(titles)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Batteries ") // Note that spaces are intentional in here
                    .title_style(Style::default()),
            )
            .select(self.tabs.index())
            .style(Style::default().fg(Color::Cyan))
            .highlight_style(Style::default().fg(Color::White));
        frame.render_widget(tabs, area);
    }

    pub fn draw_state_of_charge_bar(&self, frame: &mut Frame, area: Rect) {
        let value = f64::from(self.view.battery().state_of_charge().get::<ratio>());
        let value_label = f64::from(self.view.battery().state_of_charge().get::<percent>());

        // create blocks for gauge and text
        let gauge_block = Block::default()
            .title(" State of charge ")
            .title_style(Style::default())
            .borders(Borders::ALL & !Borders::RIGHT);
        let text_block = Block::default().borders(Borders::ALL & !Borders::LEFT);

        // allocate areas for blocks
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Min(0),
                    Constraint::Length(("|100.00 %|".len()) as u16),
                ]
                .as_ref(),
            )
            .split(area);

        let (gauge_area, text_area) = (chunks[0], chunks[1]);

        // set text and gauge colors
        let gauge_color = match () {
            _ if value > 0.3 => Color::Green,
            _ if value > 0.15 => Color::Yellow,
            _ => Color::Red,
        };
        let text_color = match () {
            _ if gauge_color == Color::Green => Color::Gray,
            _ => gauge_color,
        };

        // create colored text with separator from gauge
        let text = Text::from(vec![Line::from(vec![
            Span::raw(" "),
            Span::styled(
                format!("{:>6.2} %", value_label),
                Style::default().fg(text_color),
            ),
        ])]);

        // render components
        frame.render_widget(
            Gauge::default()
                .block(gauge_block)
                .ratio(value)
                .gauge_style(Style::default().fg(gauge_color).bg(Color::Black))
                .label(""),
            gauge_area,
        );
        frame.render_widget(
            Paragraph::new(text)
                .block(text_block)
                .alignment(Alignment::Right),
            text_area,
        );
    }

    pub fn draw_chart(&self, data: &ChartData, frame: &mut Frame, area: Rect) {
        let title = format!(" {} ", data.title());
        let block = Block::default()
            .title(title.as_str())
            .title_style(Style::default())
            .borders(Borders::ALL);
        let value = data.current();
        // ratatui automatically hides chart legend if it's height is higher than `chart.height / 3`.
        // Since we have 3 charts already, legend will be invisible for most monitors,
        // so instead writing value as a X axis label
        let x_axis = Axis::default()
            .title(value.as_str())
            .style(Style::default().fg(Color::Reset))
            .bounds(data.x_bounds());
        let y_labels: Vec<Line> = data.y_labels().into_iter().map(Line::from).collect();
        let y_axis = Axis::default()
            .title(data.y_title())
            .labels(y_labels)
            .bounds(data.y_bounds());

        frame.render_widget(
            Chart::new(vec![Dataset::default()
                .marker(Marker::Braille)
                .style(Style::default().fg(Color::Green))
                .data(data.points())])
            .block(block)
            .x_axis(x_axis)
            .y_axis(y_axis),
            area,
        );
    }

    fn draw_common_info(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Information ") // Note that spaces are intentional
            .title_style(Style::default())
            .borders(Borders::LEFT | Borders::TOP | Borders::RIGHT);

        let tech = format!("{}", self.view.battery().technology());
        let state = format!("{}", self.view.battery().state());
        let cycles = match self.view.battery().cycle_count() {
            Some(cycles) => format!("{}", cycles),
            None => "N/A".to_string(),
        };

        let items: Vec<[String; 2]> = vec![
            [
                "Vendor".into(),
                self.view.battery().vendor().unwrap_or("N/A").into(),
            ],
            [
                "Model".into(),
                self.view.battery().model().unwrap_or("N/A").into(),
            ],
            [
                "S/N".into(),
                self.view.battery().serial_number().unwrap_or("N/A").into(),
            ],
            ["Technology".into(), tech],
            ["Charge state".into(), state],
            ["Cycles count".into(), cycles],
        ];
        let header = ["Device", ""];

        self.draw_info_table(header, &items, block, frame, area);
    }

    fn draw_energy_info(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default().borders(Borders::LEFT | Borders::RIGHT);
        let battery = self.view.battery();
        let config = self.view.config();

        let consumption = format!(
            "{:.2} {}",
            battery.energy_rate().get::<watt>(),
            watt::abbreviation()
        );
        let voltage = format!(
            "{:.2} {}",
            battery.voltage().get::<volt>(),
            volt::abbreviation()
        );
        let capacity = format!(
            "{:.2} {}",
            battery.state_of_health().get::<percent>(),
            percent::abbreviation()
        );
        let current = match config.units() {
            Units::Human => format!(
                "{:.2} {}",
                battery.energy().get::<watt_hour>(),
                watt_hour::abbreviation()
            ),
            Units::Si => format!(
                "{:.2} {}",
                battery.energy().get::<joule>(),
                joule::abbreviation()
            ),
        };
        let last_full = match config.units() {
            Units::Human => format!(
                "{:.2} {}",
                battery.energy_full().get::<watt_hour>(),
                watt_hour::abbreviation()
            ),
            Units::Si => format!(
                "{:.2} {}",
                battery.energy_full().get::<joule>(),
                joule::abbreviation()
            ),
        };
        let full_design = match config.units() {
            Units::Human => format!(
                "{:.2} {}",
                battery.energy_full_design().get::<watt_hour>(),
                watt_hour::abbreviation()
            ),
            Units::Si => format!(
                "{:.2} {}",
                battery.energy_full_design().get::<joule>(),
                joule::abbreviation()
            ),
        };
        let consumption_label = match battery.state() {
            State::Charging => "Charging with",
            State::Discharging => "Discharging with",
            _ => "Consumption",
        };

        let items: Vec<[String; 2]> = vec![
            [consumption_label.into(), consumption],
            ["Voltage".into(), voltage],
            ["Capacity".into(), capacity],
            ["Current".into(), current],
            ["Last full".into(), last_full],
            ["Full design".into(), full_design],
        ];
        let header = ["Energy", ""];

        self.draw_info_table(header, &items, block, frame, area);
    }

    fn draw_timing_info(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default().borders(Borders::LEFT | Borders::RIGHT);
        let battery = self.view.battery();

        let time_to_full = match battery.time_to_full() {
            Some(time) => {
                humantime::format_duration(Duration::from_secs(time.get::<second>() as u64))
                    .to_string()
            }
            None => "N/A".to_string(),
        };

        let time_to_empty = match battery.time_to_empty() {
            Some(time) => {
                humantime::format_duration(Duration::from_secs(time.get::<second>() as u64))
                    .to_string()
            }
            None => "N/A".to_string(),
        };

        let items: Vec<[String; 2]> = vec![
            ["Time to full".into(), time_to_full],
            ["Time to empty".into(), time_to_empty],
        ];
        let header = ["Time", ""];

        self.draw_info_table(header, &items, block, frame, area);
    }

    fn draw_environment_info(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM);
        let battery = self.view.battery();
        let config = self.view.config();

        let temperature = match battery.temperature() {
            Some(temp) => match config.units() {
                Units::Human => format!(
                    "{:.2} {}",
                    temp.get::<degree_celsius>(),
                    degree_celsius::abbreviation()
                ),
                Units::Si => format!("{:.2} {}", temp.get::<kelvin>(), kelvin::abbreviation()),
            },
            None => "N/A".to_string(),
        };

        let items: Vec<[String; 2]> = vec![["Temperature".into(), temperature]];
        let header = ["Environment", ""];

        self.draw_info_table(header, &items, block, frame, area);
    }

    fn draw_info_table(
        &self,
        header: [&str; 2],
        items: &[[String; 2]],
        block: Block,
        frame: &mut Frame,
        area: Rect,
    ) {
        let header_row = Row::new(header.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .style(Style::default().add_modifier(Modifier::BOLD));

        let rows: Vec<Row> = items
            .iter()
            .map(|item| Row::new(item.iter().map(|s| s.as_str()).collect::<Vec<_>>()))
            .collect();

        let table = Table::new(rows, [Constraint::Length(17), Constraint::Length(17)])
            .header(header_row)
            .block(block);

        frame.render_widget(table, area);
    }
}

impl<'i> Deref for Painter<'i> {
    type Target = Context<'i>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
