use zellij_tile::prelude::*;

use std::convert::TryFrom;
use std::path::Path;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug)]
struct PathMetadata {
    git_worktree_root: PathBuf,
}

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

    /// Whether the stable tab-id workaround is enabled.
    ///
    /// Upstream issue:
    /// - <https://github.com/zellij-org/zellij/issues/3535>
    ///
    /// Background:
    /// Zellij's plugin action for tab rename currently targets internal tab
    /// indices, while plugin state exposes tab positions. After tab closes,
    /// these can diverge, and position-based rename calls can hit the wrong tab.
    use_stable_tab_ids: bool,

    /// Maps pane id to tab position (0-indexed), used to maintain stable tab ids
    pane_to_tab_position: BTreeMap<u32, usize>,

    /// Maps pane id to a synthetic stable tab id for `rename_tab()`.
    ///
    /// This is a local workaround for zellij-org/zellij#3535 in downstream
    /// plugins until upstream behavior is fixed.
    pane_to_stable_tab_id: BTreeMap<u32, u32>,
}

register_plugin!(State);

impl Default for State {
    fn default() -> Self {
        Self {
            userspace_configuration: BTreeMap::new(),
            tabs: vec![],
            panes: PaneManifest::default(),
            pane_working_dirs: BTreeMap::new(),
            permissions: None,
            path_metadata: BTreeMap::new(),
            use_stable_tab_ids: true,
            pane_to_tab_position: BTreeMap::new(),
            pane_to_stable_tab_id: BTreeMap::new(),
        }
    }
}

fn rem_first_and_last(value: &str) -> &str {
    let mut chars = value.chars();
    chars.next();
    chars.next_back();
    chars.as_str()
}

fn parse_bool(value: &str) -> Option<bool> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("true")
        || value.eq_ignore_ascii_case("yes")
        || value.eq_ignore_ascii_case("on")
        || value == "1"
    {
        return Some(true);
    }

    if value.eq_ignore_ascii_case("false")
        || value.eq_ignore_ascii_case("no")
        || value.eq_ignore_ascii_case("off")
        || value == "0"
    {
        return Some(false);
    }

    None
}

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.userspace_configuration = configuration;
        self.use_stable_tab_ids = self
            .userspace_configuration
            .get("use_stable_tab_ids")
            .map_or(true, |value| {
                parse_bool(value).unwrap_or_else(|| {
                    eprintln!(
                        "Invalid value for use_stable_tab_ids: {value:?}. Falling back to true."
                    );
                    true
                })
            });
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

        let Ok(reported_pane_id) = rem_first_and_last(parts[0]).parse::<u32>() else {
            eprintln!("Failed to parse pane id: {}", parts[0]);
            return false;
        };

        let pwd = rem_first_and_last(parts[1]);
        let pane_id = self.resolve_pipe_pane_id(reported_pane_id);

        self.pane_working_dirs
            .insert(pane_id, pwd.to_string().into());

        self.rebuild_stable_tab_ids();
        self.organize();

        false
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::TabUpdate(tab_info) => {
                self.tabs = tab_info;
                self.rebuild_stable_tab_ids();
            }
            Event::PaneUpdate(data) => {
                self.panes = data;
                self.rebuild_stable_tab_ids();
            }
            Event::PaneClosed(pane_id_enum) => {
                let pane_id = match pane_id_enum {
                    PaneId::Terminal(pane_id) | PaneId::Plugin(pane_id) => pane_id,
                };

                self.panes.panes = self
                    .panes
                    .panes
                    .clone()
                    .into_iter()
                    .map(|(tab_index, panes)| {
                        (
                            tab_index,
                            panes.into_iter().filter(|p| p.id != pane_id).collect(),
                        )
                    })
                    .collect();

                self.pane_working_dirs.remove(&pane_id);
                self.rebuild_stable_tab_ids();
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
    fn resolve_pipe_pane_id(&self, reported_pane_id: u32) -> u32 {
        let terminal_panes: Vec<&PaneInfo> = self
            .panes
            .panes
            .values()
            .flat_map(|pane_list| pane_list.iter())
            .filter(|pane| !pane.is_plugin && !pane.is_suppressed)
            .collect();

        if terminal_panes.iter().any(|pane| pane.id == reported_pane_id) {
            return reported_pane_id;
        }

        let focused_panes: Vec<&PaneInfo> = terminal_panes
            .into_iter()
            .filter(|pane| pane.is_focused)
            .collect();

        if focused_panes.len() == 1 {
            let focused_pane_id = focused_panes[0].id;
            eprintln!(
                "Falling back from reported pane id {reported_pane_id} to focused pane id \
                 {focused_pane_id}"
            );
            return focused_pane_id;
        }

        reported_pane_id
    }

    fn rebuild_stable_tab_ids(&mut self) {
        let mut current_pane_to_tab_position = BTreeMap::new();

        for tab in &self.tabs {
            if let Some(pane_list) = self.panes.panes.get(&tab.position) {
                for pane in pane_list {
                    if !pane.is_plugin && !pane.is_suppressed {
                        current_pane_to_tab_position.insert(pane.id, tab.position);
                    }
                }
            }
        }

        if !self.use_stable_tab_ids {
            self.pane_to_tab_position = current_pane_to_tab_position;
            return;
        }

        // WORKAROUND for https://github.com/zellij-org/zellij/issues/3535:
        // Track our own stable tab ids by pinning panes in the same tab to the
        // same synthetic id, then use that id for rename_tab() targets.
        //
        // The plugin API exposes tab positions, but rename_tab() effectively
        // behaves like it expects stable internal indices.

        // If pane ids change, transfer the stable id from the deleted pane
        // to the new pane that appeared in the same tab position.
        let mut new_panes_by_position: BTreeMap<usize, Vec<u32>> = BTreeMap::new();
        for (&pane_id, &tab_position) in &current_pane_to_tab_position {
            if !self.pane_to_stable_tab_id.contains_key(&pane_id) {
                new_panes_by_position
                    .entry(tab_position)
                    .or_default()
                    .push(pane_id);
            }
        }

        for (old_pane_id, old_tab_position) in &self.pane_to_tab_position {
            if current_pane_to_tab_position.contains_key(old_pane_id) {
                continue;
            }

            let Some(stable_id) = self.pane_to_stable_tab_id.get(old_pane_id).copied() else {
                continue;
            };

            if let Some(new_panes) = new_panes_by_position.get_mut(old_tab_position) {
                if let Some(new_pane_id) = new_panes.pop() {
                    self.pane_to_stable_tab_id.insert(new_pane_id, stable_id);
                }
            }
        }

        // Remove stale pane ids.
        self.pane_to_stable_tab_id
            .retain(|pane_id, _| current_pane_to_tab_position.contains_key(pane_id));

        // Ensure all panes in a tab share the same stable id.
        for tab in &self.tabs {
            let Some(pane_list) = self.panes.panes.get(&tab.position) else {
                continue;
            };

            let pane_ids_in_tab: Vec<u32> = pane_list
                .iter()
                .filter(|pane| !pane.is_plugin && !pane.is_suppressed)
                .map(|pane| pane.id)
                .collect();

            if pane_ids_in_tab.is_empty() {
                continue;
            }

            let stable_id = pane_ids_in_tab
                .iter()
                .find_map(|pane_id| self.pane_to_stable_tab_id.get(pane_id).copied())
                .unwrap_or_else(|| {
                    self.pane_to_stable_tab_id
                        .values()
                        .copied()
                        .max()
                        .unwrap_or(0)
                        + 1
                });

            for pane_id in pane_ids_in_tab {
                self.pane_to_stable_tab_id.insert(pane_id, stable_id);
            }
        }

        self.pane_to_tab_position = current_pane_to_tab_position;
    }

    fn rename_target_for_tab(&self, tab_position: usize, panes: &[PaneInfo]) -> Option<u32> {
        if self.use_stable_tab_ids {
            // WORKAROUND mode:
            // resolve tab target through our synthetic stable id map
            // (see zellij-org/zellij#3535).
            panes
                .iter()
                .find_map(|pane| self.pane_to_stable_tab_id.get(&pane.id).copied())
        } else {
            // Legacy mode:
            // use the tab position directly (1-indexed), which can mis-target
            // tabs after tab closures because of zellij-org/zellij#3535.
            u32::try_from(tab_position)
                .ok()
                .map(|position| position + 1)
        }
    }

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
                .get(&tab_position)
                .cloned()
                .unwrap_or_default()
                .into_iter()
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

            let Some(rename_target) = self.rename_target_for_tab(tab_position, &panes) else {
                continue;
            };

            rename_tab(rename_target, tab_name);
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
