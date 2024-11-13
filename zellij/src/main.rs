use zellij_tile::prelude::*;

use std::convert::TryFrom;
use std::path::Path;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug)]
struct PathMetadata {
    git_worktree_root: PathBuf,
}

#[derive(Default)]
struct State {
    /// The configuration passed to the plugin from zellij
    userspace_configuration: BTreeMap<String, String>,

    /// The tabs currently open in the terminal, set by the `TabUpdate` event
    tabs: Vec<TabInfo>,

    /// The panes currently open in the terminal, set by the `PaneUpdate` event
    panes: PaneManifest,

    /// Maps pane id to the working dir open in the pane
    pane_working_dirs: BTreeMap<u32, PathBuf>,

    /// Whether the plugin has the necessary permissions
    permissions: Option<PermissionStatus>,

    /// Metadata about paths
    path_metadata: BTreeMap<PathBuf, PathMetadata>,
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
            PermissionType::RunCommands,
        ]);
        subscribe(&[
            EventType::TabUpdate,
            EventType::PaneUpdate,
            EventType::PermissionRequestResult,
            EventType::RunCommandResult,
        ]);
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        eprintln!("pipe_message: {pipe_message:?}");

        if pipe_message.name != "tabula" {
            return false;
        }

        let Some(payload) = pipe_message.payload else {
            eprintln!("Expected payload, got none");
            return false;
        };

        let parts: Vec<&str> = payload.split(' ').collect();

        if parts.len() != 2 {
            eprintln!("Expected exactly 2 parts, got {}", parts.len());
            return false;
        }

        let Ok(pane_id) = rem_first_and_last(parts[0]).parse::<u32>() else {
            eprintln!("Failed to parse pane id: {}", parts[0]);
            return false;
        };

        let pwd = rem_first_and_last(parts[1]);

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
            Event::PermissionRequestResult(status) => {
                self.permissions = Some(status);
            }
            Event::RunCommandResult(exit_code, stdout, stderr, context) => {
                if context.get("plugin") != Some(&String::from("tabula")) {
                    return false;
                }

                let Some(fn_name) = context.get("fn") else {
                    eprintln!("Expected fn in context, got none");
                    return false;
                };

                if exit_code != Some(0) {
                    eprintln!(
                        "Failed to run {}: exit_code: {:?}, stdout: {:?}, stderr: {:?}",
                        fn_name,
                        exit_code,
                        String::from_utf8(stdout),
                        String::from_utf8(stderr)
                    );

                    return false;
                }

                let Ok(stdout) = String::from_utf8(stdout) else {
                    eprintln!("Failed to parse stdout for {fn_name}");
                    return false;
                };

                let stdout = stdout.trim();

                if fn_name != "get_git_worktree_root" {
                    eprintln!("Unexpected fn: {fn_name}");
                    return false;
                }

                let Some(path) = context.get("path") else {
                    eprintln!("Expected path in context, got none");
                    return false;
                };

                let path = PathBuf::from(path);

                let git_worktree_root = PathBuf::from(stdout);

                self.path_metadata
                    .insert(path, PathMetadata { git_worktree_root });

                self.organize();
            }
            _ => (),
        };

        false
    }

    fn render(&mut self, _rows: usize, _cols: usize) {}
}

impl State {
    fn get_git_worktree_root(&self, path: PathBuf) -> Option<PathBuf> {
        if let Some(metadata) = self.path_metadata.get(&path) {
            Some(metadata.git_worktree_root.clone())
        } else {
            if let Some(PermissionStatus::Granted) = self.permissions {
                let mut context = BTreeMap::new();
                context.insert(String::from("plugin"), String::from("tabula"));
                context.insert(String::from("fn"), String::from("get_git_worktree_root"));
                context.insert(String::from("path"), String::from(path.to_string_lossy()));
                run_command_with_env_variables_and_cwd(
                    &["git", "rev-parse", "--show-toplevel"],
                    BTreeMap::new(),
                    path,
                    context,
                );
            }

            None
        }
    }

    fn organize(&self) {
        'tab: for tab in &self.tabs {
            let tab_position = tab.position;

            let panes: Vec<PaneInfo> = self
                .panes
                .panes
                .clone()
                .into_iter()
                .filter(|(tab_index, _)| tab_index == &tab_position)
                .flat_map(|(_, p)| p)
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
                let Some(first_working_dir) = working_dirs_in_tab.first().copied() else {
                    // If there are no working dirs, skip this tab
                    continue 'tab;
                };

                if working_dirs_in_tab.len() == 1 {
                    break 'tab_name self.format_path(first_working_dir);
                }

                // If all working_dirs_in_tab are the same, use that as the tab name
                if working_dirs_in_tab
                    .iter()
                    .all(|dir| *dir == first_working_dir)
                {
                    break 'tab_name format!(
                        "{}/",
                        self.format_path(first_working_dir).trim_end_matches('/')
                    );
                }

                // Get the common directory of all entries in working_dirs_in_tab
                let mut common_dir = first_working_dir.clone();

                for dir in &working_dirs_in_tab {
                    while !dir.starts_with(&common_dir) {
                        if let Some(parent) = common_dir.parent() {
                            common_dir = parent.to_path_buf();
                        } else {
                            break;
                        }
                    }
                }

                format!(
                    "{}/* ({} panes)",
                    self.format_path(&common_dir).trim_end_matches('/'),
                    panes.len()
                )
            };

            if self.tabs[tab_position].name == tab_name {
                continue;
            }

            if let Ok(tab_position) = u32::try_from(tab_position) {
                rename_tab(tab_position + 1, tab_name);
            }
        }
    }

    fn format_path(&self, path: &Path) -> String {
        let git_root_dir = self.get_git_worktree_root(path.to_path_buf());

        let result = format!("{}", path.display());

        if let Some(git_root_dir) = git_root_dir {
            if let Some(git_root_dir_str) = git_root_dir.to_str() {
                if path.starts_with(git_root_dir_str) {
                    if let Some(git_root_basename) = git_root_dir.file_name() {
                        if let Some(git_root_basename) = git_root_basename.to_str() {
                            return result.replacen(git_root_dir_str, git_root_basename, 1);
                        }
                    }
                }
            }
        }

        if let Some(home_dir) = self.userspace_configuration.get("home_dir") {
            let home_dir = home_dir.trim_end_matches('/');
            if path.starts_with(home_dir) {
                return format!("~{}", result.trim_start_matches(home_dir));
            }
        }

        result
    }
}
