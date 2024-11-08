## Development

_Note_: you will need to have `wasm32-wasi` added to rust as a target to build the plugin. This can be done with `rustup target add wasm32-wasi`.

_Note_: The dev zellij layout assumes you have the following in your zshrc: (see https://superuser.com/a/230090 for explanation)

```
if [[ $1 == eval ]]
then
    "$@"
set --
fi
```

## Inside Zellij

You can load the `./plugin-dev-workspace.kdl` file as a Zellij layout to get a terminal development environment:

Either when starting Zellij:

```
zellij --layout ./plugin-dev-workspace.kdl
```

_Note that in this case there's a small bug where the plugin is opened twice, it can be remedied by closing the oldest instance or loading with the new-tab action as secified below - this will be addressed in the near future_

Or from a running Zellij session:

```bash
zellij action new-tab --layout ./plugin-dev-workspace.kdl
```

## Otherwise

1. Build the project: `cargo build` inside the `./zellij` directory
2. Load it inside a running Zellij session: `zellij action start-or-reload-plugin file:target/wasm32-wasi/debug/rust-plugin-example.wasm`
3. Repeat on changes (perhaps with a `watchexec` or similar command to run on fs changes).
