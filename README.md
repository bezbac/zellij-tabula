# zellij-tabula

A [Zellij](https://zellij.dev) plugin to automatically rename tabs based on the working directory of the contained panes.

### 🚧 Disclaimer

This project is currently under development and may be subject to frequent changes. Features may be added, modified, or removed without notice. Please use at your own risk and feel free to submit any feedback or suggestions. Thank you for your understanding.

## Installation

**Requires Zellij `0.44.0` or newer**.

### Installing the Zellij plugin

Add the following to your [zellij config](https://zellij.dev/documentation/configuration.html), replacing `YOUR_HOME_DIRECTORY` with the absolute path of your home directory:

```kdl
load_plugins {
    "https://github.com/bezbac/zellij-tabula/releases/download/v0.4.0/zellij-tabula.wasm" {
        home_dir "YOUR_HOME_DIRECTORY"
        worktree_name_display "repo_and_worktree"
        worktree_name_preview_length "10"
    }
}
```

## Configuration

### `home_dir`

Absolute path to your home directory. This is used to shorten non-git paths to `~`.

### `worktree_name_display`

Controls how linked git worktrees are displayed.

- `repo_and_worktree`: `repo/src (🌲 feature-branch...)`
- `worktree_only`: `feature-branch/src`

The default is `repo_and_worktree`.

### `worktree_name_preview_length`

Controls truncation of the displayed worktree name when `worktree_name_display` is `repo_and_worktree`.

- `0` or omitted: show the full worktree name
- positive integer: show the first `N` characters and append `...` only when truncation happens

`worktree_name_preview_length` appends `...` only when truncation happens.

Examples:

- `worktree_name_display "repo_and_worktree"` with `worktree_name_preview_length "10"` => `repo/src (🌲 feature-bra...)`
- `worktree_name_display "worktree_only"` ignores `worktree_name_preview_length` => `feature-branch/src`

## Pane Status Tracking

zellij-tabula also supports setting a pane's status via [Zellij pipes](https://zellij.dev/documentation/zellij-pipes). This allows external tools to indicate when a pane is waiting for user input.

### Usage

Send a message to the `tabula` pipe with the format:

```bash
zellij pipe --name tabula -- "status '<pane_id>' '<status>'"
```

- `<pane_id>`: The target pane's ID (from `$ZELLIJ_PANE_ID`)
- `<status>`: Either `waiting` or `none`

Set the current pane's status to `waiting`:

```bash
zellij pipe --name tabula -- "status '${ZELLIJ_PANE_ID}' 'waiting'"
```

## Integrations

- **[opencode](https://opencode.ai)** — see [`integrations/opencode/`](./integrations/opencode/) for a plugin that shows a waiting indicator when opencode requests permission.

Missing your favorite tool? [Suggest an integration](https://github.com/bezbac/zellij-tabula/issues/new?title=Integration+request+[tool+name])

## Contributing

Feel free to suggest ideas or report issues by [opening an issue](https://github.com/bezbac/zellij-tabula/issues/new).  
If you want to contribute code changes you will find some useful information in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

The content of this repository is licensed under the BSD-3-Clause license. See the [LICENSE](LICENSE) file for details.

## Acknowledgments

This plugin is based on the Zellij's [rust-example-plugin](https://github.com/zellij-org/rust-plugin-example).
