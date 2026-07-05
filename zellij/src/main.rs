use zellij_tile::prelude::*;

use std::convert::TryFrom;
use std::path::Path;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Debug)]
struct PathMetadata {
    git_worktree_root: PathBuf,
    repo_name: String,
    worktree_name: String,
}

enum WorktreeNameDisplay {
    /// Show the repository path and append the linked worktree name separately.
    RepoAndWorktree,
    /// Replace the repository name in the rendered path with the worktree name.
    WorktreeOnly,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum PaneStatus {
    #[default]
    None,
    Waiting,
}

fn format_path(state: &State, path: &Path, path_suffix: &str) -> String {
    let git_metadata = state.get_git_path_metadata(path.to_path_buf());

    let result = format!("{}", path.display());

    if let Some(git_metadata) = git_metadata {
        if let Ok(relative_path) = path.strip_prefix(&git_metadata.git_worktree_root) {
            let is_linked_worktree = git_metadata.worktree_name != git_metadata.repo_name;

            if is_linked_worktree {
                match state.worktree_name_display() {
                    WorktreeNameDisplay::RepoAndWorktree => {
                        let worktree_name = truncate_with_ellipsis(
                            &git_metadata.worktree_name,
                            state.worktree_name_preview_length(),
                        );
                        let path = if relative_path.as_os_str().is_empty() {
                            git_metadata.repo_name.clone()
                        } else {
                            format!("{}/{}", git_metadata.repo_name, relative_path.display())
                        };

                        return format!("{path}{path_suffix} (🌲 {worktree_name})");
                    }
                    WorktreeNameDisplay::WorktreeOnly => {
                        let path = if relative_path.as_os_str().is_empty() {
                            git_metadata.worktree_name.clone()
                        } else {
                            format!("{}/{}", git_metadata.worktree_name, relative_path.display())
                        };

                        return format!("{path}{path_suffix}");
                    }
                }
            }

            let path = if relative_path.as_os_str().is_empty() {
                git_metadata.repo_name.clone()
            } else {
                format!("{}/{}", git_metadata.repo_name, relative_path.display())
            };

            return format!("{path}{path_suffix}");
        }
    }

    if let Some(home_dir) = state.userspace_configuration.get("home_dir") {
        let home_dir = home_dir.trim_end_matches('/');
        if path.starts_with(home_dir) {
            return format!("~{}{}", result.trim_start_matches(home_dir), path_suffix);
        }
    }

    format!("{result}{path_suffix}")
}

fn truncate_with_ellipsis(value: &str, preview_length: usize) -> String {
    if preview_length == 0 {
        return value.to_string();
    }

    let truncated_value: String = value.chars().take(preview_length).collect();

    if truncated_value.chars().count() == value.chars().count() {
        return truncated_value;
    }

    format!("{truncated_value}...")
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

    /// Maps pane id to its current status.
    pane_statuses: BTreeMap<u32, PaneStatus>,

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

fn parse_pane_status(value: &str) -> Option<PaneStatus> {
    match value {
        "none" => Some(PaneStatus::None),
        "waiting" => Some(PaneStatus::Waiting),
        _ => None,
    }
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
            EventType::PaneClosed,
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

        if payload.starts_with("status ") {
            let parts: Vec<&str> = payload.split(' ').collect();

            if parts.len() != 3 {
                eprintln!(
                    "Expected exactly 3 parts for status update, got {}",
                    parts.len()
                );
                return false;
            }

            let Ok(reported_pane_id) = rem_first_and_last(parts[1]).parse::<u32>() else {
                eprintln!("Failed to parse pane id: {}", parts[1]);
                return false;
            };

            let Some(pane_status) = parse_pane_status(rem_first_and_last(parts[2])) else {
                let value = rem_first_and_last(parts[2]);
                eprintln!("Unknown pane status: {value}");
                return false;
            };

            let pane_id = self.resolve_pipe_pane_id(reported_pane_id);

            self.pane_statuses.insert(pane_id, pane_status);
            self.organize();

            return false;
        }

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
            Event::PaneClosed(pane_id_enum) => {
                self.handle_pane_closed(pane_id_enum);
            }
            Event::PermissionRequestResult(status) => {
                self.permissions = Some(status);
            }
            Event::RunCommandResult(exit_code, stdout, stderr, context) => {
                return self.handle_run_command_result(exit_code, stdout, stderr, &context);
            }
            _ => (),
        }

        false
    }

    fn render(&mut self, _rows: usize, _cols: usize) {}
}

impl State {
    fn handle_pane_closed(&mut self, pane_id_enum: PaneId) {
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
        self.pane_statuses.remove(&pane_id);
        self.organize();
    }

    fn handle_run_command_result(
        &mut self,
        exit_code: Option<i32>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        context: &BTreeMap<String, String>,
    ) -> bool {
        if context.get("plugin") != Some(&String::from("tabula")) {
            return false;
        }

        let Some((path, metadata)) =
            Self::parse_git_path_metadata_output(exit_code, stdout, stderr, context)
        else {
            return false;
        };

        self.path_metadata.insert(path, metadata);
        self.organize();

        false
    }

    fn parse_git_path_metadata_output(
        exit_code: Option<i32>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        context: &BTreeMap<String, String>,
    ) -> Option<(PathBuf, PathMetadata)> {
        let Some(fn_name) = context.get("fn") else {
            eprintln!("Expected fn in context, got none");
            return None;
        };

        if exit_code != Some(0) {
            eprintln!(
                "Failed to run {}: exit_code: {:?}, stdout: {:?}, stderr: {:?}",
                fn_name,
                exit_code,
                String::from_utf8(stdout),
                String::from_utf8(stderr)
            );

            return None;
        }

        let Ok(stdout) = String::from_utf8(stdout) else {
            eprintln!("Failed to parse stdout for {fn_name}");
            return None;
        };

        if fn_name != "get_git_path_metadata" {
            eprintln!("Unexpected fn: {fn_name}");
            return None;
        }

        let Some(path) = context.get("path") else {
            eprintln!("Expected path in context, got none");
            return None;
        };

        let mut stdout_lines = stdout.trim().lines();

        let Some(git_worktree_root) = stdout_lines.next().map(PathBuf::from) else {
            eprintln!("Expected git worktree root for {fn_name}");
            return None;
        };

        let Some(git_common_dir) = stdout_lines.next().map(PathBuf::from) else {
            eprintln!("Expected git common dir for {fn_name}");
            return None;
        };

        let fallback_repo_name = git_worktree_root
            .file_name()
            .and_then(|repo_name| repo_name.to_str())
            .map(str::to_owned);

        let repo_name = git_common_dir
            .parent()
            .and_then(|repo_dir| repo_dir.file_name())
            .and_then(|repo_name| repo_name.to_str())
            .map(str::to_owned)
            .or(fallback_repo_name);

        let Some(repo_name) = repo_name else {
            eprintln!("Expected repo name for {fn_name}");
            return None;
        };

        let Some(worktree_name) = git_worktree_root
            .file_name()
            .and_then(|worktree_name| worktree_name.to_str())
            .map(str::to_owned)
        else {
            eprintln!("Expected worktree name for {fn_name}");
            return None;
        };

        Some((
            PathBuf::from(path),
            PathMetadata {
                git_worktree_root,
                repo_name,
                worktree_name,
            },
        ))
    }

    fn resolve_pipe_pane_id(&self, reported_pane_id: u32) -> u32 {
        let terminal_panes: Vec<&PaneInfo> = self
            .panes
            .panes
            .values()
            .flat_map(|pane_list| pane_list.iter())
            .filter(|pane| !pane.is_plugin && !pane.is_suppressed)
            .collect();

        if terminal_panes
            .iter()
            .any(|pane| pane.id == reported_pane_id)
        {
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

    fn get_git_path_metadata(&self, path: PathBuf) -> Option<PathMetadata> {
        if let Some(metadata) = self.path_metadata.get(&path) {
            Some(metadata.clone())
        } else {
            if let Some(PermissionStatus::Granted) = self.permissions {
                let mut context = BTreeMap::new();
                context.insert(String::from("plugin"), String::from("tabula"));
                context.insert(String::from("fn"), String::from("get_git_path_metadata"));
                context.insert(String::from("path"), String::from(path.to_string_lossy()));
                run_command_with_env_variables_and_cwd(
                    &[
                        "git",
                        "rev-parse",
                        "--path-format=absolute",
                        "--show-toplevel",
                        "--git-common-dir",
                    ],
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

            let mut tab_name = 'tab_name: {
                let Some(first_working_dir) = working_dirs_in_tab.first().copied() else {
                    // If there are no working dirs, skip this tab
                    continue 'tab;
                };

                if working_dirs_in_tab.len() == 1 {
                    break 'tab_name format_path(self, first_working_dir, "");
                }

                // If all working_dirs_in_tab are the same, use that as the tab name
                if working_dirs_in_tab
                    .iter()
                    .all(|dir| *dir == first_working_dir)
                {
                    break 'tab_name format_path(self, first_working_dir, "/");
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
                    "{} ({} panes)",
                    format_path(self, &common_dir, "/*"),
                    panes.len()
                )
            };

            if panes
                .iter()
                .any(|pane| self.pane_statuses.get(&pane.id) == Some(&PaneStatus::Waiting))
            {
                tab_name = format!("⏳{tab_name}");
            }

            if self.tabs[tab_position].name == tab_name {
                continue;
            }

            let Some(rename_target) = u64::try_from(tab.tab_id).ok() else {
                continue;
            };

            rename_tab_with_id(rename_target, tab_name);
        }
    }

    fn worktree_name_display(&self) -> WorktreeNameDisplay {
        match self
            .userspace_configuration
            .get("worktree_name_display")
            .map(String::as_str)
        {
            Some("worktree_only") => WorktreeNameDisplay::WorktreeOnly,
            _ => WorktreeNameDisplay::RepoAndWorktree,
        }
    }

    fn worktree_name_preview_length(&self) -> usize {
        self.userspace_configuration
            .get("worktree_name_preview_length")
            .and_then(|preview_length| preview_length.parse::<usize>().ok())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_home_dir(home_dir: &str) -> State {
        let mut state = State::default();
        state
            .userspace_configuration
            .insert(String::from("home_dir"), home_dir.to_string());
        state
    }

    fn state_with_worktree_config(display: &str, preview_length: usize) -> State {
        let mut state = State::default();
        state
            .userspace_configuration
            .insert(String::from("worktree_name_display"), display.to_string());
        state.userspace_configuration.insert(
            String::from("worktree_name_preview_length"),
            preview_length.to_string(),
        );
        state
    }

    #[test]
    fn formats_home_relative_paths() {
        let state = state_with_home_dir("/home/alice");

        assert_eq!(
            format_path(&state, Path::new("/home/alice/project/src"), ""),
            "~/project/src"
        );
    }

    #[test]
    fn formats_main_checkout_paths_with_repo_name() {
        let mut state = State::default();
        state.path_metadata.insert(
            PathBuf::from("/home/alice/git-project/src"),
            PathMetadata {
                git_worktree_root: PathBuf::from("/home/alice/git-project"),
                repo_name: "git-project".to_string(),
                worktree_name: "git-project".to_string(),
            },
        );

        assert_eq!(
            format_path(&state, Path::new("/home/alice/git-project/src"), ""),
            "git-project/src"
        );
    }

    #[test]
    fn formats_repo_and_worktree_paths_with_truncation() {
        let mut state = state_with_worktree_config("repo_and_worktree", 10);
        state.path_metadata.insert(
            PathBuf::from("/home/alice/git-project-worktree/src"),
            PathMetadata {
                git_worktree_root: PathBuf::from("/home/alice/git-project-worktree"),
                repo_name: "git-project".to_string(),
                worktree_name: "git-project-worktree".to_string(),
            },
        );

        assert_eq!(
            format_path(
                &state,
                Path::new("/home/alice/git-project-worktree/src"),
                ""
            ),
            "git-project/src (🌲 git-projec...)"
        );
    }

    #[test]
    fn formats_worktree_only_paths_without_emoji() {
        let mut state = state_with_worktree_config("worktree_only", 10);
        state.path_metadata.insert(
            PathBuf::from("/home/alice/git-project-worktree/src"),
            PathMetadata {
                git_worktree_root: PathBuf::from("/home/alice/git-project-worktree"),
                repo_name: "git-project".to_string(),
                worktree_name: "git-project-worktree".to_string(),
            },
        );

        assert_eq!(
            format_path(
                &state,
                Path::new("/home/alice/git-project-worktree/src"),
                ""
            ),
            "git-project-worktree/src"
        );
    }

    #[test]
    fn does_not_add_ellipsis_when_worktree_name_fits_preview_length() {
        let mut state = state_with_worktree_config("repo_and_worktree", 10);
        state.path_metadata.insert(
            PathBuf::from("/home/alice/feature-x/src"),
            PathMetadata {
                git_worktree_root: PathBuf::from("/home/alice/feature-x"),
                repo_name: "git-project".to_string(),
                worktree_name: "feature-x".to_string(),
            },
        );

        assert_eq!(
            format_path(&state, Path::new("/home/alice/feature-x/src"), ""),
            "git-project/src (🌲 feature-x)"
        );
    }

    #[test]
    fn puts_multi_pane_suffix_before_worktree_annotation() {
        let mut state = state_with_worktree_config("repo_and_worktree", 10);
        state.path_metadata.insert(
            PathBuf::from("/home/alice/git-project-worktree/src"),
            PathMetadata {
                git_worktree_root: PathBuf::from("/home/alice/git-project-worktree"),
                repo_name: "git-project".to_string(),
                worktree_name: "git-project-worktree".to_string(),
            },
        );

        assert_eq!(
            format_path(
                &state,
                Path::new("/home/alice/git-project-worktree/src"),
                "/*"
            ),
            "git-project/src/* (🌲 git-projec...)"
        );
    }

    #[test]
    fn parses_pane_status_values() {
        assert_eq!(parse_pane_status("waiting"), Some(PaneStatus::Waiting));
        assert_eq!(parse_pane_status("none"), Some(PaneStatus::None));
        assert_eq!(parse_pane_status("busy"), None);
    }
}
