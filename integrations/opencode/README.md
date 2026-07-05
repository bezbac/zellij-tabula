# Opencode Integration

When opencode asks for permission (e.g. to run a command or edit a file), this plugin sets the zellij tabula pane status to `waiting` so you can see at a glance that the agent is blocked on your input. Once you respond, the status is cleared back to `none`.

## Installation

```bash
mkdir -p ~/.config/opencode/plugins
cp .opencode/plugins/zellij-tabula.ts ~/.config/opencode/plugins/
```
