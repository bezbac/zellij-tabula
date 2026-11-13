use zellij_tile::prelude::*;

use regex::Regex;
use std::convert::TryFrom;
use std::path::Path;
use std::str::FromStr;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

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

/// A pane's working directory is not always known the moment its pane update
/// arrives (the shell process may not have reported it yet), so seeding is
/// retried a bounded number of times until every pane has one.
const SEED_RETRY_INTERVAL_SECS: f64 = 0.1;
const MAX_SEED_RETRIES: u32 = 20;

#[derive(Default)]
struct SeedOutcome {
    /// At least one pane was given a working directory this round.
    seeded: bool,
    /// At least one pane's working directory could not be read yet.
    pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RewriteTarget {
    /// Applies to the linked worktree name, shown after the repo path (e.g. `(🌲 worktree)`).
    Worktree,
    /// Applies to the git-root basename, which is always shown first.
    Repo,
    /// Applies to every individual path segment.
    Segment,
    /// Applies to the entire final tab name.
    Path,
}

impl RewriteTarget {
    fn try_from_name(name: &str) -> Result<RewriteTarget, ParseRewriteError> {
        match name {
            "worktree" => Ok(Self::Worktree),
            "repo" => Ok(Self::Repo),
            "segment" => Ok(Self::Segment),
            "path" => Ok(Self::Path),
            other => Err(ParseRewriteError::UnknownTarget(other.to_string())),
        }
    }
}

struct Rewrite {
    target: RewriteTarget,
    pattern: Regex,
    replacement: String,
}

#[derive(Debug)]
enum ParseRewriteError {
    UnknownTarget(String),
    ArgCount {
        target: RewriteTarget,
        expected: usize,
        got: usize,
    },
    MissingPattern(RewriteTarget),
    MissingReplacement(RewriteTarget),
    InvalidRegex {
        target: RewriteTarget,
        pattern: String,
    },
}

impl std::fmt::Display for ParseRewriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTarget(name) => write!(f, "Unknown rewrite target: {name}"),
            Self::ArgCount {
                target,
                expected,
                got,
            } => write!(
                f,
                "Expected {expected} arguments (pattern, replacement) for {target:?}, got {got}"
            ),
            Self::MissingPattern(target) => write!(f, "Expected string pattern for {target:?}"),
            Self::MissingReplacement(target) => {
                write!(f, "Expected string replacement for {target:?}")
            }
            Self::InvalidRegex { target, pattern } => {
                write!(f, "Invalid regex for {target:?}: {pattern}")
            }
        }
    }
}

impl Rewrite {
    fn try_from_kdl_node(node: &kdl::KdlNode) -> Result<Rewrite, ParseRewriteError> {
        let name = node.name().to_string();

        let target = RewriteTarget::try_from_name(&name)?;

        let entries = node.entries();

        if entries.len() != 2 {
            return Err(ParseRewriteError::ArgCount {
                target,
                expected: 2,
                got: entries.len(),
            });
        }

        let Some(pattern) = entries[0].value().as_string() else {
            return Err(ParseRewriteError::MissingPattern(target));
        };

        let Some(replacement) = entries[1].value().as_string() else {
            return Err(ParseRewriteError::MissingReplacement(target));
        };

        let Ok(pattern) = Regex::new(pattern) else {
            return Err(ParseRewriteError::InvalidRegex {
                target,
                pattern: pattern.to_string(),
            });
        };

        Ok(Rewrite {
            target,
            pattern,
            replacement: replacement.to_string(),
        })
    }
}

fn format_path(state: &State, path: &Path, path_suffix: &str) -> String {
    let git_metadata = state.get_git_path_metadata(path.to_path_buf());

    let result = format!("{}", path.display());

    if let Some(git_metadata) = git_metadata {
        if let Ok(relative_path) = path.strip_prefix(&git_metadata.git_worktree_root) {
            let is_linked_worktree = git_metadata.worktree_name != git_metadata.repo_name;

            let repo_name = apply_rewrites(
                &git_metadata.repo_name,
                RewriteTarget::Repo,
                &state.rewrites,
            );

            if is_linked_worktree {
                match state.worktree_name_display() {
                    WorktreeNameDisplay::RepoAndWorktree => {
                        let worktree_name = apply_rewrites(
                            &git_metadata.worktree_name,
                            RewriteTarget::Worktree,
                            &state.rewrites,
                        );
                        let worktree_name = truncate_with_ellipsis(
                            &worktree_name,
                            state.worktree_name_preview_length(),
                        );
                        let path = if relative_path.as_os_str().is_empty() {
                            repo_name.clone()
                        } else {
                            format!("{repo_name}/{}", relative_path.display())
                        };
                        let path = apply_segment_rewrites(&path, &state.rewrites);

                        return apply_path_rewrites(
                            &format!("{path}{path_suffix} (🌲 {worktree_name})"),
                            &state.rewrites,
                        );
                    }
                    WorktreeNameDisplay::WorktreeOnly => {
                        let worktree_name = apply_rewrites(
                            &git_metadata.worktree_name,
                            RewriteTarget::Worktree,
                            &state.rewrites,
                        );
                        let path = if relative_path.as_os_str().is_empty() {
                            worktree_name.clone()
                        } else {
                            format!("{worktree_name}/{}", relative_path.display())
                        };
                        let path = apply_segment_rewrites(&path, &state.rewrites);

                        return apply_path_rewrites(
                            &format!("{path}{path_suffix}"),
                            &state.rewrites,
                        );
                    }
                }
            }

            let path = if relative_path.as_os_str().is_empty() {
                repo_name.clone()
            } else {
                format!("{repo_name}/{}", relative_path.display())
            };
            let path = apply_segment_rewrites(&path, &state.rewrites);

            return apply_path_rewrites(&format!("{path}{path_suffix}"), &state.rewrites);
        }
    }

    if let Some(home_dir) = state.userspace_configuration.get("home_dir") {
        let home_dir = home_dir.trim_end_matches('/');
        if path.starts_with(home_dir) {
            let name = format!("~{}{}", result.trim_start_matches(home_dir), path_suffix);
            return apply_path_rewrites(&name, &state.rewrites);
        }
    }

    apply_path_rewrites(&format!("{result}{path_suffix}"), &state.rewrites)
}

fn apply_rewrites(name: &str, target: RewriteTarget, rewrites: &[Rewrite]) -> String {
    let mut out = name.to_string();

    for rule in rewrites {
        if rule.target == target {
            out = rule
                .pattern
                .replace_all(&out, rule.replacement.as_str())
                .into_owned();
        }
    }

    out
}

fn apply_segment_rewrites(name: &str, rewrites: &[Rewrite]) -> String {
    let segment_rules: Vec<&Rewrite> = rewrites
        .iter()
        .filter(|rule| rule.target == RewriteTarget::Segment)
        .collect();

    if segment_rules.is_empty() {
        return name.to_string();
    }

    name.split('/')
        .map(|segment| {
            let mut rewritten = segment.to_string();

            for rule in &segment_rules {
                rewritten = rule
                    .pattern
                    .replace_all(&rewritten, rule.replacement.as_str())
                    .into_owned();
            }

            rewritten
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn apply_path_rewrites(name: &str, rewrites: &[Rewrite]) -> String {
    let mut out = name.to_string();

    for rule in rewrites {
        if rule.target == RewriteTarget::Path {
            out = rule
                .pattern
                .replace_all(&out, rule.replacement.as_str())
                .into_owned();
        }
    }

    out
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

/// How a tab whose panes are in different directories gets its name.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum RenamePolicy {
    /// Name after the deepest directory shared by all panes (`dir/* (N panes)`).
    #[default]
    Common,
    /// Name after the directory more than half of the panes are in.
    Majority,
    /// Name after the directory of the pane covering the largest area.
    Area,
}

impl RenamePolicy {
    fn from_config(value: Option<&str>) -> RenamePolicy {
        match value {
            None | Some("common") => Self::Common,
            Some("majority") => Self::Majority,
            Some("area") => Self::Area,
            Some(other) => {
                eprintln!("Unknown rename_policy {other:?}, falling back to \"common\"");
                Self::Common
            }
        }
    }
}

/// A pane's working directory along with what the rename policies need to know about it.
#[derive(Debug)]
struct PaneDir<'a> {
    dir: &'a PathBuf,
    /// Rows times columns, including the frame.
    area: usize,
    is_floating: bool,
}

/// The directory a tab is named after, and how it was picked.
#[derive(Debug, PartialEq, Eq)]
enum TabDir {
    /// A single pane: `dir`
    Single(PathBuf),
    /// Several panes in this exact directory: `dir/`
    Shared(PathBuf),
    /// Panes spread across directories, this is their deepest common one: `dir/* (N panes)`
    Common(PathBuf),
}

/// Picks the directory to name a tab after. `None` if no pane has a known working directory.
fn choose_tab_dir(policy: RenamePolicy, panes: &[PaneDir]) -> Option<TabDir> {
    let first = panes.first()?.dir;

    if panes.len() == 1 {
        return Some(TabDir::Single(first.clone()));
    }

    if panes.iter().all(|pane| pane.dir == first) {
        return Some(TabDir::Shared(first.clone()));
    }

    let picked = match policy {
        RenamePolicy::Common => None,
        RenamePolicy::Majority => majority_dir(panes).map(|dir| TabDir::Shared(dir.clone())),
        RenamePolicy::Area => largest_pane_dir(panes).map(|dir| TabDir::Single(dir.clone())),
    };

    // Policies without a clear winner (e.g. a 2/2 split) fall back to the common directory
    Some(picked.unwrap_or_else(|| TabDir::Common(common_dir(panes))))
}

/// The directory strictly more than half of the panes are in (2 of 3, 3 of 5, ...).
fn majority_dir<'a>(panes: &[PaneDir<'a>]) -> Option<&'a PathBuf> {
    let mut counts: BTreeMap<&PathBuf, usize> = BTreeMap::new();

    for pane in panes {
        *counts.entry(pane.dir).or_default() += 1;
    }

    counts
        .into_iter()
        .find(|(_, count)| count * 2 > panes.len())
        .map(|(dir, _)| dir)
}

/// The directory of the single largest pane. Floating panes only compete if there are no tiled
/// ones, as they are drawn on top of the tiled panes rather than taking up space of their own.
fn largest_pane_dir<'a>(panes: &[PaneDir<'a>]) -> Option<&'a PathBuf> {
    let has_tiled = panes.iter().any(|pane| !pane.is_floating);
    let candidates: Vec<&PaneDir> = panes
        .iter()
        .filter(|pane| !has_tiled || !pane.is_floating)
        .collect();

    let max_area = candidates.iter().map(|pane| pane.area).max()?;
    let mut largest = candidates.iter().filter(|pane| pane.area == max_area);

    let winner = largest.next()?;
    let other_largest_dir = largest.any(|pane| pane.dir != winner.dir);

    // An exact tie between directories has no winner
    (!other_largest_dir).then_some(winner.dir)
}

/// The deepest directory containing every pane's working directory.
fn common_dir(panes: &[PaneDir]) -> PathBuf {
    let mut common = panes
        .first()
        .map(|pane| pane.dir.clone())
        .unwrap_or_default();

    for pane in panes {
        while !pane.dir.starts_with(&common) {
            if let Some(parent) = common.parent() {
                common = parent.to_path_buf();
            } else {
                break;
            }
        }
    }

    common
}

/// Names zellij gives tabs by default (`Tab #3`), or an empty name.
fn is_default_tab_name(name: &str) -> bool {
    name.is_empty()
        || name
            .strip_prefix("Tab #")
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

/// How many not yet applied rename requests to remember per tab.
const MAX_PENDING_RENAMES: usize = 8;

/// A `git rev-parse` lookup for a path that hasn't produced metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GitLookup {
    /// The command is still running.
    Pending,
    /// The command failed, the path is not inside a git repository.
    // NOTE: never retried, so a later `git init` in that path only shows up after a reload
    NotARepo,
}

/// What a pane looks like for naming purposes: id, size, and whether it floats or is hidden.
type PaneShape = (u32, usize, usize, bool, bool);

/// The shape of all panes per tab position. Unlike the full manifest, this doesn't change
/// when only the focus moves.
fn pane_layout(manifest: &PaneManifest) -> BTreeMap<usize, Vec<PaneShape>> {
    manifest
        .panes
        .iter()
        .map(|(position, panes)| {
            let shapes = panes
                .iter()
                .map(|pane| {
                    (
                        pane.id,
                        pane.pane_rows,
                        pane.pane_columns,
                        pane.is_floating,
                        pane.is_suppressed,
                    )
                })
                .collect();
            (*position, shapes)
        })
        .collect()
}

#[derive(Default)]
struct State {
    /// The configuration passed to the plugin from zellij
    userspace_configuration: BTreeMap<String, String>,

    /// User-defined string rewrites, parsed from the `rewrites` config block
    rewrites: Vec<Rewrite>,

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

    /// Remaining attempts to seed pane working dirs that are not known yet.
    seed_retries: u32,

    /// Whether a retry timer for seeding pane working dirs is pending.
    seed_retry_scheduled: bool,

    /// How tabs with panes in different directories are named, from `rename_policy`.
    rename_policy: RenamePolicy,

    /// Whether manually renamed tabs are left alone, from `respect_rename`.
    respect_rename: bool,

    /// Renames this plugin requested that haven't shown up in a tab update yet, per tab id,
    /// oldest first.
    pending_renames: BTreeMap<usize, Vec<String>>,

    /// The last name per tab id that this plugin set and saw applied.
    applied_names: BTreeMap<usize, String>,

    /// Git lookups that are running or failed, so they aren't repeated on every rename. In a
    /// `RefCell` because lookups are started while formatting names, which only borrows `State`.
    git_lookups: RefCell<BTreeMap<PathBuf, GitLookup>>,

    /// Ids of tabs that were renamed by someone else and are no longer named automatically.
    /// Only kept in memory, so a plugin reload or session restore names them automatically again.
    manually_renamed_tabs: BTreeSet<usize>,
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
        self.rewrites = parse_rewrites(&configuration);
        self.rename_policy =
            RenamePolicy::from_config(configuration.get("rename_policy").map(String::as_str));
        self.respect_rename = configuration
            .get("respect_rename")
            .is_some_and(|value| value == "true");
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
            EventType::CwdChanged,
            EventType::Timer,
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

        if payload.starts_with("status ") {
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

        false
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::TabUpdate(tab_info) => {
                let released = self.track_renames(&tab_info);
                self.tabs = tab_info;
                if released {
                    self.organize();
                }
            }
            Event::PaneUpdate(data) => {
                // Pane updates also arrive for focus changes, only rename when the panes were
                // added, removed or resized (which matters for the `area` policy)
                let layout_changed = pane_layout(&self.panes) != pane_layout(&data);
                self.panes = data;
                self.seed_retries = 0;
                let organized = self.attempt_seed();
                if layout_changed && !organized {
                    self.organize();
                }
            }
            Event::PaneClosed(pane_id_enum) => {
                self.handle_pane_closed(pane_id_enum);
            }
            Event::CwdChanged(pane_id_enum, cwd, _) => {
                let pane_id = match pane_id_enum {
                    PaneId::Terminal(id) | PaneId::Plugin(id) => id,
                };
                self.pane_working_dirs.insert(pane_id, cwd);
                self.organize();
            }
            Event::PermissionRequestResult(status) => {
                self.permissions = Some(status);
                self.seed_retries = 0;
                self.attempt_seed();
            }
            Event::Timer(_) => {
                self.seed_retry_scheduled = false;
                self.attempt_seed();
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
    /// Returns whether the tabs were renamed.
    fn attempt_seed(&mut self) -> bool {
        let outcome = self.seed_pane_working_dirs();

        if outcome.seeded {
            self.organize();
        }

        if outcome.pending {
            self.schedule_seed_retry();
        }

        outcome.seeded
    }

    fn schedule_seed_retry(&mut self) {
        if self.seed_retry_scheduled || self.seed_retries >= MAX_SEED_RETRIES {
            return;
        }

        self.seed_retry_scheduled = true;
        self.seed_retries += 1;
        set_timeout(SEED_RETRY_INTERVAL_SECS);
    }

    fn seed_pane_working_dirs(&mut self) -> SeedOutcome {
        if self.permissions != Some(PermissionStatus::Granted) {
            return SeedOutcome::default();
        }

        let mut outcome = SeedOutcome::default();

        for pane in self.panes.panes.values().flat_map(|panes| panes.iter()) {
            if pane.is_plugin || pane.is_suppressed {
                continue;
            }

            if self.pane_working_dirs.contains_key(&pane.id) {
                continue;
            }

            match get_pane_cwd(PaneId::Terminal(pane.id)) {
                Ok(cwd) => {
                    self.pane_working_dirs.insert(pane.id, cwd);
                    outcome.seeded = true;
                }
                Err(_) => outcome.pending = true,
            }
        }

        outcome
    }

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

        let lookup_path = context.get("path").map(PathBuf::from);

        let Some((path, metadata)) =
            Self::parse_git_path_metadata_output(exit_code, stdout, stderr, context)
        else {
            // Remember the failure so the lookup isn't repeated on every rename
            if let Some(path) = lookup_path {
                self.git_lookups
                    .borrow_mut()
                    .insert(path, GitLookup::NotARepo);
            }
            return false;
        };

        self.git_lookups.borrow_mut().remove(&path);
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
            // Already running, or known to not be inside a git repository
            if self.git_lookups.borrow().contains_key(&path) {
                return None;
            }

            if let Some(PermissionStatus::Granted) = self.permissions {
                self.git_lookups
                    .borrow_mut()
                    .insert(path.clone(), GitLookup::Pending);
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

    /// Renames every tab whose name is out of date, and remembers what was requested so
    /// `track_renames` can tell these renames apart from manual ones.
    fn organize(&mut self) {
        for (tab_id, tab_name) in self.tab_renames() {
            let Ok(rename_target) = u64::try_from(tab_id) else {
                continue;
            };

            let pending = self.pending_renames.entry(tab_id).or_default();

            // This rename was already requested and hasn't shown up in a tab update yet
            if pending.last() == Some(&tab_name) {
                continue;
            }

            pending.push(tab_name.clone());
            if pending.len() > MAX_PENDING_RENAMES {
                pending.remove(0);
            }

            rename_tab_with_id(rename_target, tab_name);
        }
    }

    /// The new names for all tabs that need renaming, as `(tab_id, name)`.
    fn tab_renames(&self) -> Vec<(usize, String)> {
        let mut renames = Vec::new();

        for tab in &self.tabs {
            if self.manually_renamed_tabs.contains(&tab.tab_id) {
                continue;
            }

            let panes: Vec<PaneInfo> = self
                .panes
                .panes
                .get(&tab.position)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|p| !p.is_suppressed && !p.is_plugin)
                .collect();

            let pane_dirs: Vec<PaneDir> = panes
                .iter()
                .filter_map(|pane| {
                    Some(PaneDir {
                        dir: self.pane_working_dirs.get(&pane.id)?,
                        area: pane.pane_rows * pane.pane_columns,
                        is_floating: pane.is_floating,
                    })
                })
                .collect();

            // Skip tabs where no working dirs are known yet
            let Some(tab_dir) = choose_tab_dir(self.rename_policy, &pane_dirs) else {
                continue;
            };

            let mut tab_name = match tab_dir {
                TabDir::Single(dir) => format_path(self, &dir, ""),
                TabDir::Shared(dir) => format_path(self, &dir, "/"),
                TabDir::Common(dir) => {
                    format!("{} ({} panes)", format_path(self, &dir, "/*"), panes.len())
                }
            };

            if panes
                .iter()
                .any(|pane| self.pane_statuses.get(&pane.id) == Some(&PaneStatus::Waiting))
            {
                tab_name = format!("⏳{tab_name}");
            }

            if tab.name != tab_name {
                renames.push((tab.tab_id, tab_name));
            }
        }

        renames
    }

    /// Compares incoming tab names with the previous ones to find tabs that were renamed by
    /// someone other than this plugin. With `respect_rename`, those are no longer renamed until
    /// they get a default (`Tab #N`) or empty name again.
    ///
    /// Returns whether a tab went back to automatic naming and should be renamed now.
    fn track_renames(&mut self, new_tabs: &[TabInfo]) -> bool {
        let mut released = false;

        for tab in new_tabs {
            let id = tab.tab_id;

            // Our most recent request was applied
            if self.pending_renames.get(&id).and_then(|names| names.last()) == Some(&tab.name) {
                self.pending_renames.remove(&id);
                self.applied_names.insert(id, tab.name.clone());
                released |= self.manually_renamed_tabs.remove(&id);
                continue;
            }

            // Our name was restored, e.g. by cancelling a rename
            if self.applied_names.get(&id) == Some(&tab.name) {
                released |= self.manually_renamed_tabs.remove(&id);
                continue;
            }

            if !self.respect_rename {
                continue;
            }

            if is_default_tab_name(&tab.name) {
                released |= self.manually_renamed_tabs.remove(&id);
                continue;
            }

            let Some(previous) = self.tabs.iter().find(|t| t.tab_id == id) else {
                // A new tab, or the first update since the plugin loaded
                continue;
            };

            let changed = previous.name != tab.name;
            // An older request that is only now being applied
            let ours = self
                .pending_renames
                .get(&id)
                .is_some_and(|names| names.contains(&tab.name));

            if changed && !ours {
                self.manually_renamed_tabs.insert(id);
            }
        }

        let open: BTreeSet<usize> = new_tabs.iter().map(|tab| tab.tab_id).collect();
        self.manually_renamed_tabs.retain(|id| open.contains(id));
        self.pending_renames.retain(|id, _| open.contains(id));
        self.applied_names.retain(|id, _| open.contains(id));

        released
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

    // HACK: zellij provides this host function when running as a plugin. Defining it here lets
    // the unit tests link natively (`cargo test --target <host triple>`).
    #[cfg(not(target_arch = "wasm32"))]
    #[no_mangle]
    extern "C" fn host_run_plugin_command() {}

    fn pane_dir(dir: &PathBuf, columns: usize) -> PaneDir<'_> {
        PaneDir {
            dir,
            area: columns * 10,
            is_floating: false,
        }
    }

    #[test]
    fn parses_rename_policy() {
        assert_eq!(RenamePolicy::from_config(None), RenamePolicy::Common);
        assert_eq!(
            RenamePolicy::from_config(Some("majority")),
            RenamePolicy::Majority
        );
        assert_eq!(RenamePolicy::from_config(Some("area")), RenamePolicy::Area);
        assert_eq!(
            RenamePolicy::from_config(Some("nope")),
            RenamePolicy::Common
        );
    }

    #[test]
    fn every_policy_handles_single_and_shared_dirs() {
        let repo = PathBuf::from("/home/alice/repo");

        for policy in [
            RenamePolicy::Common,
            RenamePolicy::Majority,
            RenamePolicy::Area,
        ] {
            assert_eq!(choose_tab_dir(policy, &[]), None);
            assert_eq!(
                choose_tab_dir(policy, &[pane_dir(&repo, 10)]),
                Some(TabDir::Single(repo.clone()))
            );
            assert_eq!(
                choose_tab_dir(policy, &[pane_dir(&repo, 10), pane_dir(&repo, 20)]),
                Some(TabDir::Shared(repo.clone()))
            );
        }
    }

    #[test]
    fn common_policy_uses_common_dir() {
        let repo = PathBuf::from("/home/alice/repo");
        let notes = PathBuf::from("/home/alice/notes");
        let panes = [
            pane_dir(&repo, 10),
            pane_dir(&repo, 10),
            pane_dir(&notes, 10),
        ];

        assert_eq!(
            choose_tab_dir(RenamePolicy::Common, &panes),
            Some(TabDir::Common(PathBuf::from("/home/alice")))
        );
    }

    #[test]
    fn majority_policy_needs_more_than_half() {
        let repo = PathBuf::from("/home/alice/repo");
        let notes = PathBuf::from("/home/alice/notes");
        let tmp = PathBuf::from("/tmp");

        // 2 of 3
        assert_eq!(
            choose_tab_dir(
                RenamePolicy::Majority,
                &[
                    pane_dir(&notes, 10),
                    pane_dir(&repo, 10),
                    pane_dir(&repo, 10)
                ]
            ),
            Some(TabDir::Shared(repo.clone()))
        );

        // 2 of 4 is no majority
        assert_eq!(
            choose_tab_dir(
                RenamePolicy::Majority,
                &[
                    pane_dir(&repo, 10),
                    pane_dir(&repo, 10),
                    pane_dir(&notes, 10),
                    pane_dir(&tmp, 10)
                ]
            ),
            Some(TabDir::Common(PathBuf::from("/")))
        );

        // 1 of 1 of 1
        assert_eq!(
            choose_tab_dir(
                RenamePolicy::Majority,
                &[
                    pane_dir(&repo, 10),
                    pane_dir(&notes, 10),
                    pane_dir(&tmp, 10)
                ]
            ),
            Some(TabDir::Common(PathBuf::from("/")))
        );
    }

    #[test]
    fn area_policy_picks_largest_pane() {
        let repo = PathBuf::from("/home/alice/repo");
        let notes = PathBuf::from("/home/alice/notes");
        let tmp = PathBuf::from("/tmp");

        // 60/40
        assert_eq!(
            choose_tab_dir(
                RenamePolicy::Area,
                &[pane_dir(&notes, 40), pane_dir(&repo, 60)]
            ),
            Some(TabDir::Single(repo.clone()))
        );

        // 40/30/30, even though the two smaller panes share a directory
        assert_eq!(
            choose_tab_dir(
                RenamePolicy::Area,
                &[
                    pane_dir(&repo, 40),
                    pane_dir(&notes, 30),
                    pane_dir(&notes, 30)
                ]
            ),
            Some(TabDir::Single(repo.clone()))
        );

        // A tie between directories falls back to the common dir
        assert_eq!(
            choose_tab_dir(
                RenamePolicy::Area,
                &[pane_dir(&repo, 50), pane_dir(&tmp, 50)]
            ),
            Some(TabDir::Common(PathBuf::from("/")))
        );
    }

    #[test]
    fn area_policy_ignores_floating_panes_unless_all_float() {
        let repo = PathBuf::from("/home/alice/repo");
        let notes = PathBuf::from("/home/alice/notes");
        let floating_notes = PaneDir {
            is_floating: true,
            ..pane_dir(&notes, 90)
        };

        assert_eq!(
            choose_tab_dir(RenamePolicy::Area, &[pane_dir(&repo, 30), floating_notes]),
            Some(TabDir::Single(repo.clone()))
        );

        let floating_repo = PaneDir {
            is_floating: true,
            ..pane_dir(&repo, 30)
        };
        let floating_notes = PaneDir {
            is_floating: true,
            ..pane_dir(&notes, 90)
        };
        assert_eq!(
            choose_tab_dir(RenamePolicy::Area, &[floating_repo, floating_notes]),
            Some(TabDir::Single(notes.clone()))
        );
    }

    #[test]
    fn recognizes_default_tab_names() {
        assert!(is_default_tab_name("Tab #1"));
        assert!(is_default_tab_name("Tab #12"));
        assert!(is_default_tab_name(""));
        assert!(!is_default_tab_name("Tab #"));
        assert!(!is_default_tab_name("Tab #1a"));
        assert!(!is_default_tab_name("repo/"));
    }

    fn tab(tab_id: usize, name: &str) -> TabInfo {
        TabInfo {
            tab_id,
            position: tab_id,
            name: name.to_string(),
            ..TabInfo::default()
        }
    }

    /// A state with a single tab at position/id 0 that has one pane in `/home/alice/repo`.
    fn state_with_repo_tab(respect_rename: bool, tab_name: &str) -> State {
        let mut state = state_with_home_dir("/home/alice");
        state.respect_rename = respect_rename;
        state.tabs = vec![tab(0, tab_name)];
        state.panes.panes.insert(
            0,
            vec![PaneInfo {
                id: 1,
                ..PaneInfo::default()
            }],
        );
        state
            .pane_working_dirs
            .insert(1, PathBuf::from("/home/alice/repo"));
        state
    }

    /// Feeds a tab update to the state, the way `update` does.
    fn tab_update(state: &mut State, tabs: Vec<TabInfo>) -> bool {
        let released = state.track_renames(&tabs);
        state.tabs = tabs;
        released
    }

    #[test]
    fn renames_tabs_after_their_panes() {
        let state = state_with_repo_tab(true, "Tab #1");
        assert_eq!(state.tab_renames(), vec![(0, String::from("~/repo"))]);
    }

    #[test]
    fn manual_rename_is_respected() {
        let mut state = state_with_repo_tab(true, "Tab #1");
        state.organize();
        assert!(!tab_update(&mut state, vec![tab(0, "~/repo")]));
        assert!(state.manually_renamed_tabs.is_empty());

        // The user renames the tab
        assert!(!tab_update(&mut state, vec![tab(0, "my tab")]));
        assert!(state.manually_renamed_tabs.contains(&0));

        // ... and it is left alone when the pane moves
        state
            .pane_working_dirs
            .insert(1, PathBuf::from("/home/alice/other"));
        assert!(state.tab_renames().is_empty());
    }

    #[test]
    fn manual_rename_is_ignored_without_respect_rename() {
        let mut state = state_with_repo_tab(false, "~/repo");
        tab_update(&mut state, vec![tab(0, "my tab")]);

        assert!(state.manually_renamed_tabs.is_empty());
        assert_eq!(state.tab_renames(), vec![(0, String::from("~/repo"))]);
    }

    #[test]
    fn late_applied_own_rename_is_not_manual() {
        let mut state = state_with_repo_tab(true, "Tab #1");
        state.organize(); // requests "~/repo"
        state
            .pane_working_dirs
            .insert(1, PathBuf::from("/home/alice/other"));
        state.organize(); // requests "~/other" before "~/repo" was applied

        tab_update(&mut state, vec![tab(0, "~/repo")]);
        assert!(state.manually_renamed_tabs.is_empty());

        tab_update(&mut state, vec![tab(0, "~/other")]);
        assert!(state.manually_renamed_tabs.is_empty());
        assert!(state.pending_renames.is_empty());
        assert_eq!(state.applied_names.get(&0), Some(&String::from("~/other")));
    }

    #[test]
    fn in_flight_rename_is_not_sent_again() {
        let mut state = state_with_repo_tab(true, "Tab #1");
        state.organize();
        state.organize();

        assert_eq!(
            state.pending_renames.get(&0),
            Some(&vec![String::from("~/repo")])
        );
    }

    #[test]
    fn manual_rename_is_overridden_without_respect_rename() {
        let mut state = state_with_repo_tab(false, "Tab #1");
        state.organize();
        tab_update(&mut state, vec![tab(0, "~/repo")]);
        tab_update(&mut state, vec![tab(0, "my tab")]);

        // Not considered in flight anymore, so the name is requested again
        state.organize();
        assert_eq!(
            state.pending_renames.get(&0),
            Some(&vec![String::from("~/repo")])
        );
    }

    #[test]
    fn default_name_releases_manual_rename() {
        let mut state = state_with_repo_tab(true, "~/repo");
        tab_update(&mut state, vec![tab(0, "my tab")]);
        assert!(state.manually_renamed_tabs.contains(&0));

        assert!(tab_update(&mut state, vec![tab(0, "Tab #1")]));
        assert!(state.manually_renamed_tabs.is_empty());
        assert_eq!(state.tab_renames(), vec![(0, String::from("~/repo"))]);
    }

    #[test]
    fn cancelled_rename_releases_tab() {
        let mut state = state_with_repo_tab(true, "Tab #1");
        state.organize();
        tab_update(&mut state, vec![tab(0, "~/repo")]);

        // Typing in rename mode updates the name with every key press
        tab_update(&mut state, vec![tab(0, "m")]);
        tab_update(&mut state, vec![tab(0, "my")]);
        assert!(state.manually_renamed_tabs.contains(&0));

        // Cancelling restores the name this plugin set
        assert!(tab_update(&mut state, vec![tab(0, "~/repo")]));
        assert!(state.manually_renamed_tabs.is_empty());
    }

    #[test]
    fn closed_tabs_are_forgotten() {
        let mut state = state_with_repo_tab(true, "~/repo");
        tab_update(&mut state, vec![tab(0, "my tab")]);
        tab_update(&mut state, vec![]);

        assert!(state.manually_renamed_tabs.is_empty());
        assert!(state.pending_renames.is_empty());
        assert!(state.applied_names.is_empty());
    }

    fn sized_pane(id: u32, columns: usize, is_focused: bool) -> PaneInfo {
        PaneInfo {
            id,
            pane_rows: 10,
            pane_columns: columns,
            is_focused,
            ..PaneInfo::default()
        }
    }

    fn manifest(panes: Vec<PaneInfo>) -> PaneManifest {
        PaneManifest {
            panes: std::iter::once((0, panes)).collect(),
        }
    }

    #[test]
    fn pane_layout_ignores_focus_but_not_size() {
        let before = manifest(vec![sized_pane(1, 50, true), sized_pane(2, 50, false)]);
        let refocused = manifest(vec![sized_pane(1, 50, false), sized_pane(2, 50, true)]);
        let resized = manifest(vec![sized_pane(1, 60, true), sized_pane(2, 40, false)]);

        assert_eq!(pane_layout(&before), pane_layout(&refocused));
        assert_ne!(pane_layout(&before), pane_layout(&resized));
    }

    /// Two panes in different dirs in one tab, already named after the equal split.
    fn state_with_split_tab(policy: RenamePolicy) -> State {
        let mut state = state_with_home_dir("/home/alice");
        state.rename_policy = policy;
        state.tabs = vec![tab(0, "~/* (2 panes)")];
        state.panes = manifest(vec![sized_pane(1, 50, true), sized_pane(2, 50, false)]);
        state
            .pane_working_dirs
            .insert(1, PathBuf::from("/home/alice/repo"));
        state
            .pane_working_dirs
            .insert(2, PathBuf::from("/home/alice/notes"));
        state
    }

    #[test]
    fn resizing_panes_renames_tab() {
        let mut state = state_with_split_tab(RenamePolicy::Area);
        assert!(state.tab_renames().is_empty());

        state.update(Event::PaneUpdate(manifest(vec![
            sized_pane(1, 60, true),
            sized_pane(2, 40, false),
        ])));

        assert_eq!(
            state.pending_renames.get(&0),
            Some(&vec![String::from("~/repo")])
        );
    }

    #[test]
    fn focus_change_does_not_rename_tab() {
        let mut state = state_with_split_tab(RenamePolicy::Area);
        // Make the stored name outdated, so a rename would show up if one happened
        state.tabs = vec![tab(0, "outdated")];

        state.update(Event::PaneUpdate(manifest(vec![
            sized_pane(1, 50, false),
            sized_pane(2, 50, true),
        ])));

        assert!(state.pending_renames.is_empty());
    }

    #[test]
    fn git_lookups_are_not_repeated() {
        let mut state = State {
            permissions: Some(PermissionStatus::Granted),
            ..State::default()
        };
        let path = PathBuf::from("/tmp/not-a-repo");

        assert!(state.get_git_path_metadata(path.clone()).is_none());
        assert_eq!(
            state.git_lookups.borrow().get(&path),
            Some(&GitLookup::Pending)
        );

        let context: BTreeMap<String, String> = [
            ("plugin", "tabula"),
            ("fn", "get_git_path_metadata"),
            ("path", "/tmp/not-a-repo"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        state.handle_run_command_result(Some(128), vec![], b"fatal".to_vec(), &context);

        assert_eq!(
            state.git_lookups.borrow().get(&path),
            Some(&GitLookup::NotARepo)
        );
        assert!(state.get_git_path_metadata(path.clone()).is_none());
        assert_eq!(
            state.git_lookups.borrow().get(&path),
            Some(&GitLookup::NotARepo)
        );
    }

    #[test]
    fn successful_git_lookup_is_cached_as_metadata() {
        let mut state = State {
            permissions: Some(PermissionStatus::Granted),
            ..State::default()
        };
        let path = PathBuf::from("/home/alice/repo");
        state.get_git_path_metadata(path.clone());

        let context: BTreeMap<String, String> = [
            ("plugin", "tabula"),
            ("fn", "get_git_path_metadata"),
            ("path", "/home/alice/repo"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        state.handle_run_command_result(
            Some(0),
            b"/home/alice/repo\n/home/alice/repo/.git\n".to_vec(),
            vec![],
            &context,
        );

        assert!(state.git_lookups.borrow().is_empty());
        assert_eq!(
            state
                .get_git_path_metadata(path)
                .map(|metadata| metadata.repo_name),
            Some(String::from("repo"))
        );
    }

    #[test]
    fn parses_pane_status_values() {
        assert_eq!(parse_pane_status("waiting"), Some(PaneStatus::Waiting));
        assert_eq!(parse_pane_status("none"), Some(PaneStatus::None));
        assert_eq!(parse_pane_status("busy"), None);
    }
}

fn parse_rewrites(configuration: &BTreeMap<String, String>) -> Vec<Rewrite> {
    let Some(raw) = configuration.get("rewrites") else {
        return Vec::new();
    };

    let Ok(document) = kdl::KdlDocument::from_str(raw) else {
        eprintln!("Failed to parse rewrites: {raw}");
        return Vec::new();
    };

    let mut rewrites = Vec::new();

    for node in document.nodes() {
        match Rewrite::try_from_kdl_node(node) {
            Ok(rewrite) => rewrites.push(rewrite),
            Err(message) => eprintln!("{message}"),
        }
    }

    rewrites
}
