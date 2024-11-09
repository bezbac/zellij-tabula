use ansi_term::{Colour::Fixed, Style};
use zellij_tile::prelude::*;

use std::{collections::BTreeMap, path::PathBuf};

#[derive(Default)]
struct State {
    tabs: Vec<TabInfo>,
    panes: PaneManifest,
    userspace_configuration: BTreeMap<String, String>,

    /// Maps pane id to the working dir open in the pane
    pane_working_dirs: BTreeMap<u32, PathBuf>,
}

register_plugin!(State);

fn rem_first_and_last(value: &str) -> &str {
    let mut chars = value.chars();
    chars.next();
    chars.next_back();
    chars.as_str()
}

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.userspace_configuration = configuration;
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);
        subscribe(&[EventType::TabUpdate, EventType::PaneUpdate]);
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        eprintln!("pipe_message: {:?}", pipe_message);

        if pipe_message.name != "tabula" {
            return false;
        }

        if pipe_message.payload.is_none() {
            eprintln!("Expected payload, got none");
            return false;
        }

        let payload = pipe_message.payload.unwrap();

        let mut parts: Vec<&str> = payload.split(" ").collect();

        if parts.len() != 2 {
            eprintln!("Expected exactly 2 parts, got {}", parts.len());
            return false;
        }

        let mut iter = parts.into_iter();

        let pane_id = {
            rem_first_and_last(iter.next().unwrap())
                .parse::<u32>()
                .unwrap()
        };
        let pwd = rem_first_and_last(iter.next().unwrap());

        self.pane_working_dirs
            .insert(pane_id, pwd.to_string().into());

        self.organize();

        false
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::TabUpdate(tab_info) => {
                self.tabs = tab_info;
            }
            Event::PaneUpdate(data) => {
                self.panes = data;
            }
            _ => (),
        };

        self.organize();

        false
    }

    fn render(&mut self, rows: usize, cols: usize) {}
}

impl State {
    fn organize(&self) {
        'tab: for tab in &self.tabs {
            let tab_name = tab.name.to_string();
            let tab_position = tab.position;

            let panes: Vec<PaneInfo> = self
                .panes
                .panes
                .clone()
                .into_iter()
                .filter(|(tab_index, _)| tab_index == &tab_position)
                .map(|(_, p)| p)
                .flatten()
                .filter(|p| !p.is_suppressed && !p.is_plugin)
                .collect();

            let working_dirs_in_tab: Vec<&PathBuf> = panes
                .iter()
                .filter_map(|p| self.pane_working_dirs.get(&p.id))
                .collect();

            if working_dirs_in_tab.is_empty() {
                continue;
            }

            let tab_name = 'tab_name: {
                if working_dirs_in_tab.len() < 1 {
                    continue 'tab;
                }

                if working_dirs_in_tab.len() == 1 {
                    break 'tab_name format!("{}", working_dirs_in_tab.first().unwrap().display());
                }

                // If all working_dirs_in_tab are the same, use that as the tab name
                if working_dirs_in_tab
                    .iter()
                    .all(|dir| dir == working_dirs_in_tab.first().unwrap())
                {
                    break 'tab_name format!(
                        "{}/ ({} panes)",
                        working_dirs_in_tab
                            .first()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .trim_end_matches("/"),
                        panes.len()
                    );
                }

                // Get the common directory of all entries in working_dirs_in_tab
                let mut common_dir = working_dirs_in_tab.first().unwrap().clone().clone();

                for dir in &working_dirs_in_tab {
                    while !dir.starts_with(&common_dir) {
                        if let Some(parent) = common_dir.parent() {
                            common_dir = parent.to_path_buf();
                        } else {
                            break;
                        }
                    }
                }

                let common_dir_str = common_dir.to_str();

                if common_dir_str.is_none() {
                    continue 'tab;
                }

                let common_dir_str = common_dir_str.unwrap().trim_end_matches("/");

                format!("{}/* ({} panes)", common_dir_str, panes.len())
            };

            if self.tabs[tab_position].name == tab_name {
                continue;
            }

            rename_tab(tab_position as u32, tab_name);
        }
    }
}

pub const CYAN: u8 = 51;
pub const GRAY_LIGHT: u8 = 238;
pub const GRAY_DARK: u8 = 245;
pub const WHITE: u8 = 15;
pub const BLACK: u8 = 16;
pub const RED: u8 = 124;
pub const GREEN: u8 = 154;
pub const ORANGE: u8 = 166;

fn color_bold(color: u8, text: &str) -> String {
    format!("{}", Style::new().fg(Fixed(color)).bold().paint(text))
}
