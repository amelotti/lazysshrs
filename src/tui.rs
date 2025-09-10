use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::path::Path;

use crate::ssh_config::{SshConfig, SshHost};
use crate::form::HostForm;
use crate::config::AppConfig;
use crate::connectivity::ConnectivityTest;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

#[derive(PartialEq, Clone)]
pub enum AppState {
    List,
    Form,
    Edit,
    Confirm,
    ConfirmEdit,
    Search,
    Popup,
}

pub struct App {
    hosts: Vec<SshHost>,
    list_state: ListState,
    state: AppState,
    form: HostForm,
    app_config: AppConfig,
    search_query: String,
    filtered_hosts: Vec<usize>,
    matcher: SkimMatcherV2,
    editing_host_index: Option<usize>,
    popup_message: String,
    previous_state: AppState,
}

impl App {
    pub fn new(config: SshConfig, app_config: AppConfig) -> Self {
        let mut app = Self {
            hosts: config.hosts,
            list_state: ListState::default(),
            state: AppState::List,
            form: HostForm::default(),
            app_config,
            search_query: String::new(),
            filtered_hosts: Vec::new(),
            matcher: SkimMatcherV2::default(),
            editing_host_index: None,
            popup_message: String::new(),
            previous_state: AppState::List,
        };
        if !app.hosts.is_empty() {
            let first_host = app.hosts.iter().position(|h| !h.is_separator).unwrap_or(0);
            app.list_state.select(Some(first_host));
        }
        app
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_app(&mut terminal);

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    fn run_app(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                match self.state {
                    AppState::List => match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('a') => {
                            self.state = AppState::Form;
                            self.form = HostForm::default();
                            self.editing_host_index = None;
                        }
                        KeyCode::Char('e') => {
                            if let Some(selected) = self.list_state.selected() {
                                if let Some(host) = self.hosts.get(selected) {
                                    if !host.is_separator {
                                        self.load_host_for_editing(selected);
                                        self.state = AppState::Edit;
                                    }
                                }
                            }
                        }
                        KeyCode::Char('p') => {
                            if let Some(selected) = self.list_state.selected() {
                                if let Some(host) = self.hosts.get(selected).cloned() {
                                    if !host.is_separator {
                                        self.test_connectivity(&host);
                                    }
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(selected) = self.list_state.selected() {
                                if let Some(host) = self.hosts.get(selected).cloned() {
                                    if !host.is_separator {
                                        if let Err(e) = self.connect_ssh(&host) {
                                            self.previous_state = self.state.clone();
                                            self.popup_message = format!("Erro na conexão SSH: {}", e);
                                            self.state = AppState::Popup;
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('/') => {
                            self.state = AppState::Search;
                            self.search_query.clear();
                            self.update_search();
                        }
                        KeyCode::Down => self.next(),
                        KeyCode::Up => self.previous(),
                        _ => {}
                    },
                    AppState::Form | AppState::Edit => match key.code {
                        KeyCode::Esc => {
                            self.state = AppState::List;
                            self.editing_host_index = None;
                        }
                        KeyCode::Tab => self.form.next_field(),
                        KeyCode::BackTab => self.form.prev_field(),
                        KeyCode::Enter => {
                            if self.form.is_valid() {
                                self.state = if self.editing_host_index.is_some() {
                                    AppState::ConfirmEdit
                                } else {
                                    AppState::Confirm
                                };
                            }
                        }
                        KeyCode::Char(c) => {
                            let mut current = self.form.get_field(self.form.current_field).to_string();
                            current.push(c);
                            self.form.set_field(self.form.current_field, current);
                        }
                        KeyCode::Backspace => {
                            let mut current = self.form.get_field(self.form.current_field).to_string();
                            current.pop();
                            self.form.set_field(self.form.current_field, current);
                        }
                        _ => {}
                    },
                    AppState::Confirm => match key.code {
                        KeyCode::Esc => self.state = AppState::Form,
                        KeyCode::Enter => {
                            self.save_host()?;
                            self.state = AppState::List;
                            self.editing_host_index = None;
                        }
                        _ => {}
                    },
                    AppState::ConfirmEdit => match key.code {
                        KeyCode::Esc => self.state = AppState::Edit,
                        KeyCode::Enter => {
                            self.update_host()?;
                            self.state = AppState::List;
                            self.editing_host_index = None;
                        }
                        _ => {}
                    },
                    AppState::Search => match key.code {
                        KeyCode::Esc => {
                            self.state = AppState::List;
                            self.search_query.clear();
                        }
                        KeyCode::Enter => {
                            if !self.filtered_hosts.is_empty() {
                                self.list_state.select(Some(self.filtered_hosts[0]));
                            }
                            self.state = AppState::List;
                            self.search_query.clear();
                        }
                        KeyCode::Char(c) => {
                            self.search_query.push(c);
                            self.update_search();
                        }
                        KeyCode::Backspace => {
                            self.search_query.pop();
                            self.update_search();
                        }
                        KeyCode::Down => self.next_search_result(),
                        KeyCode::Up => self.prev_search_result(),
                        _ => {}
                    },
                    AppState::Popup => match key.code {
                        KeyCode::Enter | KeyCode::Esc => {
                            self.state = self.previous_state.clone();
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    fn ui(&mut self, f: &mut Frame) {
        match self.state {
            AppState::List => self.render_list(f),
            AppState::Form => self.render_form(f, "Add Host"),
            AppState::Edit => self.render_form(f, "Edit Host"),
            AppState::Confirm => self.render_confirm(f, "Confirm New Host"),
            AppState::ConfirmEdit => self.render_confirm(f, "Confirm Changes"),
            AppState::Search => self.render_search(f),
            AppState::Popup => {
                // Renderizar estado anterior como fundo
                match self.previous_state {
                    AppState::List => self.render_list(f),
                    AppState::Search => self.render_search(f),
                    _ => self.render_list(f),
                }
                // Renderizar popup por cima
                self.render_popup(f);
            }
        }
    }

    fn render_list(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(f.size());

        let items: Vec<ListItem> = self
            .hosts
            .iter()
            .map(|host| {
                if host.is_separator {
                    ListItem::new(Line::from(Span::styled(&host.name, Style::default().fg(Color::Gray))))
                } else {
                    ListItem::new(Line::from(Span::raw(&host.name)))
                }
            })
            .collect();

        let hosts_list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("SSH Hosts (Enter: connect, a: add, e: edit, p: ping, /: search)"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");

        f.render_stateful_widget(hosts_list, chunks[0], &mut self.list_state);

        let selected_host = self.list_state.selected()
            .and_then(|i| self.hosts.get(i))
            .filter(|host| !host.is_separator);

        let details = if let Some(host) = selected_host {
            let mut lines = vec![
                Line::from(vec![
                    Span::styled("Host: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&host.name),
                ]),
            ];

            if let Some(hostname) = &host.hostname {
                lines.push(Line::from(vec![
                    Span::styled("Hostname: ", Style::default().fg(Color::Yellow)),
                    Span::raw(hostname),
                ]));
            }

            if let Some(user) = &host.user {
                lines.push(Line::from(vec![
                    Span::styled("User: ", Style::default().fg(Color::Yellow)),
                    Span::raw(user),
                ]));
            }

            if let Some(port) = host.port {
                lines.push(Line::from(vec![
                    Span::styled("Port: ", Style::default().fg(Color::Yellow)),
                    Span::raw(port.to_string()),
                ]));
            }

            if let Some(identity_file) = &host.identity_file {
                lines.push(Line::from(vec![
                    Span::styled("Identity File: ", Style::default().fg(Color::Yellow)),
                    Span::raw(identity_file),
                ]));
            }

            for (key, value) in &host.other_options {
                lines.push(Line::from(vec![
                    Span::styled(format!("{}: ", key), Style::default().fg(Color::Yellow)),
                    Span::raw(value),
                ]));
            }

            Paragraph::new(lines)
        } else {
            Paragraph::new("No host selected")
        };

        let details_block = details.block(Block::default().borders(Borders::ALL).title("Host Details"));
        f.render_widget(details_block, chunks[1]);
    }

    fn render_form(&mut self, f: &mut Frame, title: &str) {
        use ratatui::widgets::{Clear, Paragraph};
        use ratatui::layout::Alignment;
        
        let area = f.size();
        f.render_widget(Clear, area);
        
        let form_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(10), Constraint::Min(0)])
            .split(area)[0];
        
        let mut lines = vec![];
        let field_names = HostForm::field_names();
        
        for (i, name) in field_names.iter().enumerate() {
            let value = self.form.get_field(i);
            let style = if i == self.form.current_field {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            
            lines.push(Line::from(vec![
                Span::styled(format!("{}: ", name), style),
                Span::styled(value, style),
            ]));
        }
        
        lines.push(Line::from(""));
        lines.push(Line::from("Tab/Shift+Tab: Navigate | Enter: OK | Esc: Cancel"));
        
        let form = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(title))
            .alignment(Alignment::Left);
        
        f.render_widget(form, form_area);
    }
    
    fn render_confirm(&mut self, f: &mut Frame, title: &str) {
        use ratatui::widgets::{Clear, Paragraph};
        use ratatui::layout::Alignment;
        
        let area = f.size();
        f.render_widget(Clear, area);
        
        let confirm_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(12), Constraint::Min(0)])
            .split(area)[0];
        
        let mut lines = vec![Line::from("Confirm host configuration:"), Line::from("")];
        let field_names = HostForm::field_names();
        
        for (i, name) in field_names.iter().enumerate() {
            let value = self.form.get_field(i);
            if !value.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled(format!("{}: ", name), Style::default().fg(Color::Yellow)),
                    Span::raw(value),
                ]));
            }
        }
        
        lines.push(Line::from(""));
        lines.push(Line::from("Enter: Save | Esc: Back to form"));
        
        let confirm = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(title))
            .alignment(Alignment::Left);
        
        f.render_widget(confirm, confirm_area);
    }
    
    fn save_host(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs::{self, OpenOptions};
        use std::io::Write;
        
        let config_path = self.app_config.get_workdir().join(&self.form.folder).join("config");
        let is_new_file = !config_path.exists();
        
        // Criar diretório se não existir
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Abrir arquivo para escrita
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config_path)?;
        
        // Escrever configuração do host
        if config_path.metadata()?.len() > 0 {
            writeln!(file)?; // Linha em branco se arquivo não estiver vazio
        }
        
        writeln!(file, "Host {}", self.form.host)?;
        writeln!(file, "    Hostname {}", self.form.hostname)?;
        writeln!(file, "    User {}", self.form.user)?;
        
        if !self.form.port.is_empty() {
            writeln!(file, "    Port {}", self.form.port)?;
        }
        if !self.form.identity_file.is_empty() {
            writeln!(file, "    IdentityFile {}", self.form.identity_file)?;
        }
        if !self.form.local_forward.is_empty() {
            writeln!(file, "    LocalForward {}", self.form.local_forward)?;
        }
        
        // Adicionar Include se for arquivo novo
        if is_new_file {
            self.add_include_to_main_config(&config_path)?;
        }
        
        Ok(())
    }
    
    fn add_include_to_main_config(&self, new_config_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs::{self, OpenOptions};
        use std::io::Write;
        
        let main_config = self.app_config.get_main_config_path();
        
        let include_line = format!("Include {}", new_config_path.display());
        
        if main_config.exists() {
            let content = fs::read_to_string(&main_config)?;
            if !content.contains(&include_line) {
                // Reescrever arquivo com Include no início
                let mut file = OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(&main_config)?;
                
                writeln!(file, "{}", include_line)?;
                if !content.is_empty() {
                    writeln!(file)?; // Linha em branco
                    write!(file, "{}", content)?;
                }
            }
        } else {
            // Criar arquivo principal se não existir
            fs::create_dir_all(main_config.parent().unwrap())?;
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .open(&main_config)?;
            writeln!(file, "{}", include_line)?;
        }
        
        Ok(())
    }

    fn next(&mut self) {
        let mut i = match self.list_state.selected() {
            Some(i) => if i >= self.hosts.len() - 1 { 0 } else { i + 1 },
            None => 0,
        };
        
        while i < self.hosts.len() && self.hosts[i].is_separator {
            i = if i >= self.hosts.len() - 1 { 0 } else { i + 1 };
        }
        
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        let mut i = match self.list_state.selected() {
            Some(i) => if i == 0 { self.hosts.len() - 1 } else { i - 1 },
            None => 0,
        };
        
        while i < self.hosts.len() && self.hosts[i].is_separator {
            i = if i == 0 { self.hosts.len() - 1 } else { i - 1 };
        }
        
        self.list_state.select(Some(i));
    }

    fn update_search(&mut self) {
        self.filtered_hosts.clear();
        
        if self.search_query.is_empty() {
            return;
        }
        
        for (i, host) in self.hosts.iter().enumerate() {
            if !host.is_separator {
                if let Some(_) = self.matcher.fuzzy_match(&host.name, &self.search_query) {
                    self.filtered_hosts.push(i);
                }
            }
        }
        
        // Ordenar por score de match
        self.filtered_hosts.sort_by(|&a, &b| {
            let score_a = self.matcher.fuzzy_match(&self.hosts[a].name, &self.search_query).unwrap_or(0);
            let score_b = self.matcher.fuzzy_match(&self.hosts[b].name, &self.search_query).unwrap_or(0);
            score_b.cmp(&score_a)
        });
    }
    
    fn next_search_result(&mut self) {
        if !self.filtered_hosts.is_empty() {
            let current = self.list_state.selected().unwrap_or(0);
            if let Some(pos) = self.filtered_hosts.iter().position(|&i| i == current) {
                let next_pos = (pos + 1) % self.filtered_hosts.len();
                self.list_state.select(Some(self.filtered_hosts[next_pos]));
            } else if !self.filtered_hosts.is_empty() {
                self.list_state.select(Some(self.filtered_hosts[0]));
            }
        }
    }
    
    fn prev_search_result(&mut self) {
        if !self.filtered_hosts.is_empty() {
            let current = self.list_state.selected().unwrap_or(0);
            if let Some(pos) = self.filtered_hosts.iter().position(|&i| i == current) {
                let prev_pos = if pos == 0 { self.filtered_hosts.len() - 1 } else { pos - 1 };
                self.list_state.select(Some(self.filtered_hosts[prev_pos]));
            } else if !self.filtered_hosts.is_empty() {
                self.list_state.select(Some(self.filtered_hosts[0]));
            }
        }
    }
    
    fn render_search(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(f.size());
        
        // Barra de busca
        let search_text = format!("Search: {}", self.search_query);
        let search_bar = Paragraph::new(search_text)
            .block(Block::default().borders(Borders::ALL).title("Fuzzy Search"))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(search_bar, chunks[0]);
        
        // Lista filtrada
        let list_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);
        
        let items: Vec<ListItem> = if self.search_query.is_empty() {
            vec![ListItem::new(Line::from("Type to search..."))]
        } else if self.filtered_hosts.is_empty() {
            vec![ListItem::new(Line::from("No matches found"))]
        } else {
            self.filtered_hosts.iter().map(|&i| {
                let host = &self.hosts[i];
                ListItem::new(Line::from(Span::raw(&host.name)))
            }).collect()
        };
        
        let hosts_list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(format!("Results ({})", self.filtered_hosts.len())))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");
        
        f.render_stateful_widget(hosts_list, list_chunks[0], &mut self.list_state);
        
        // Detalhes do host selecionado
        let selected_host = self.list_state.selected()
            .and_then(|i| self.hosts.get(i))
            .filter(|host| !host.is_separator);
        
        let details = if let Some(host) = selected_host {
            let mut lines = vec![
                Line::from(vec![
                    Span::styled("Host: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&host.name),
                ]),
            ];
            
            if let Some(hostname) = &host.hostname {
                lines.push(Line::from(vec![
                    Span::styled("Hostname: ", Style::default().fg(Color::Yellow)),
                    Span::raw(hostname),
                ]));
            }
            
            if let Some(user) = &host.user {
                lines.push(Line::from(vec![
                    Span::styled("User: ", Style::default().fg(Color::Yellow)),
                    Span::raw(user),
                ]));
            }
            
            Paragraph::new(lines)
        } else {
            Paragraph::new("No host selected")
        };
        
        let details_block = details.block(Block::default().borders(Borders::ALL).title("Host Details"));
        f.render_widget(details_block, list_chunks[1]);
        
        // Instruções
        let help_text = "↑/↓: Navigate | Enter: Select | Esc: Cancel";
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray));
        
        let help_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(f.size())[1];
        
        f.render_widget(help, help_area);
    }
    
    fn load_host_for_editing(&mut self, host_index: usize) {
        if let Some(host) = self.hosts.get(host_index) {
            self.editing_host_index = Some(host_index);
            
            // Determinar pasta baseada no source_dir ou usar pasta padrão
            let folder = host.source_dir.clone().unwrap_or_else(|| "main".to_string());
            
            self.form = HostForm {
                folder,
                host: host.name.clone(),
                hostname: host.hostname.clone().unwrap_or_default(),
                user: host.user.clone().unwrap_or_default(),
                port: host.port.map(|p| p.to_string()).unwrap_or_default(),
                identity_file: host.identity_file.clone().unwrap_or_default(),
                local_forward: host.other_options.get("localforward").cloned().unwrap_or_default(),
                current_field: 0,
            };
        }
    }
    
    fn update_host(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(host_index) = self.editing_host_index {
            // Para edição, precisamos remover o host antigo e adicionar o novo
            // Por simplicidade, vamos apenas atualizar os dados na memória
            // e depois reescrever o arquivo
            self.remove_host_from_file(host_index)?;
            self.save_host()?;
        }
        Ok(())
    }
    
    fn remove_host_from_file(&mut self, host_index: usize) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        
        if let Some(host) = self.hosts.get(host_index) {
            let source_dir = host.source_dir.clone().unwrap_or_else(|| "ssh".to_string());
            let config_path = if source_dir == "ssh" {
                self.app_config.get_main_config_path()
            } else {
                self.app_config.get_workdir().join(&source_dir).join("config")
            };
            
            if config_path.exists() {
                let content = fs::read_to_string(&config_path)?;
                let mut new_content = String::new();
                let mut lines = content.lines();
                let mut _skip_until_next_host = false;
                
                while let Some(line) = lines.next() {
                    let trimmed = line.trim();
                    
                    if trimmed.starts_with("Host ") {
                        if trimmed == format!("Host {}", host.name) {
                            // Pular linhas até o próximo Host ou fim do arquivo
                            while let Some(next_line) = lines.next() {
                                let next_trimmed = next_line.trim();
                                if next_trimmed.starts_with("Host ") {
                                    new_content.push_str(next_line);
                                    new_content.push('\n');
                                    break;
                                }
                            }
                            // Continuar processamento
                        } else {
                            new_content.push_str(line);
                            new_content.push('\n');
                        }
                    } else {
                        new_content.push_str(line);
                        new_content.push('\n');
                    }
                }
                
                fs::write(&config_path, new_content)?;
            }
        }
        
        Ok(())
    }
    
    fn test_connectivity(&mut self, host: &SshHost) {
        if let (Some(hostname), Some(port)) = (&host.hostname, host.port) {
            self.previous_state = self.state.clone();
            
            let success = ConnectivityTest::test_tcp_connection(hostname, port);
            
            self.popup_message = if success {
                format!("Host {} respondeu na porta {}", hostname, port)
            } else {
                format!("Host {} não respondeu na porta {}", hostname, port)
            };
            
            self.state = AppState::Popup;
        } else {
            self.previous_state = self.state.clone();
            self.popup_message = "Host não possui hostname ou porta configurados".to_string();
            self.state = AppState::Popup;
        }
    }
    
    fn render_popup(&mut self, f: &mut Frame) {
        use ratatui::widgets::{Clear, Paragraph};
        use ratatui::layout::Alignment;
        
        let area = f.size();
        
        // Calcular área do popup (centralizado)
        let popup_width = 60.min(area.width - 4);
        let popup_height = 5;
        let x = (area.width - popup_width) / 2;
        let y = (area.height - popup_height) / 2;
        
        let popup_area = ratatui::layout::Rect {
            x,
            y,
            width: popup_width,
            height: popup_height,
        };
        
        // Limpar área do popup
        f.render_widget(Clear, popup_area);
        
        // Renderizar popup
        let popup = Paragraph::new(self.popup_message.clone())
            .block(Block::default().borders(Borders::ALL).title("Teste de Conectividade"))
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });
        
        f.render_widget(popup, popup_area);
        
        // Renderizar instrução
        let help_area = ratatui::layout::Rect {
            x,
            y: y + popup_height,
            width: popup_width,
            height: 1,
        };
        
        let help = Paragraph::new("Enter/Esc: Fechar")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray));
        
        f.render_widget(help, help_area);
    }
    
    fn connect_ssh(&mut self, host: &SshHost) -> Result<(), Box<dyn std::error::Error>> {
        use crossterm::{
            execute,
            terminal::{disable_raw_mode, enable_raw_mode, LeaveAlternateScreen, EnterAlternateScreen},
        };
        use std::io;
        
        // Sair completamente do modo TUI
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
        
        // Executar conexão SSH
        let result = ConnectivityTest::connect_ssh(&host.name);
        
        // Restaurar modo TUI
        execute!(io::stdout(), EnterAlternateScreen)?;
        enable_raw_mode()?;
        
        result
    }
}