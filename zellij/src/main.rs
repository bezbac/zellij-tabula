use ansi_term::{Colour::Fixed, Style};
use zellij_tile::prelude::*;

use std::collections::BTreeMap;

#[derive(Default)]
struct State {
    piped_messages: Vec<PipeMessage>,
    userspace_configuration: BTreeMap<String, String>,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.userspace_configuration = configuration;
        request_permission(&[PermissionType::ReadApplicationState]);
        subscribe(&[]);
    }
    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        eprintln!("pipe_message: {:?}", pipe_message);
        self.piped_messages.push(pipe_message);
        true
    }
    fn update(&mut self, event: Event) -> bool {
        false
    }

    fn render(&mut self, rows: usize, cols: usize) {
        println!("");
        for msg in &self.piped_messages {
            println!("Piped message: {:?}", msg);
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
