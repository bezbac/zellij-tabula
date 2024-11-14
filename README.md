# zellij-tabula

A [Zellij](https://zellij.dev) plugin to automatically rename tabs based on the working directory of the contained panes.

### 🚧 Disclaimer

This project is currently under development and may be subject to frequent changes. Features may be added, modified, or removed without notice. Please use at your own risk and feel free to submit any feedback or suggestions. Thank you for your understanding.

## Installation

zellij-tabula requires both a zellij-plugin _and_ a shell plugin to function. As of right now, only zsh is supported.

**Requires Zellij `0.40.0` or never**.

### Installing the Zellij plugin

Add the following to your [zellij config](https://zellij.dev/documentation/configuration.html), replacing `YOUR_HOME_DIRECTORY` with the absolute path of your home directory:

```kdl
load_plugins {
    "https://github.com/bezbac/zellij-tabula/releases/download/v0.1.1/zellij-tabula.wasm" {
        home_dir "YOUR_HOME_DIRECTORY"
    }
}
```

### Installing the zsh plugin

<details>
  <summary>Using <a href="https://github.com/rossmacarthur/sheldon" target="_blank">sheldon</a></summary>

Add the following to your sheldon [plugins.toml](https://github.com/rossmacarthur/sheldon?tab=readme-ov-file#%EF%B8%8F-configuration) config:

```toml
[plugins.zellij-tabula]
github = "bezbac/zellij-tabula"
use = ["{{ name }}.plugin.zsh"]
tag = "v0.1.1"
```

</details>

Details for more zsh plugin managers will follow. Please [open an issue](https://github.com/bezbac/zellij-tabula/issues/new) for suggesting one.

## Contributing

Feel free to suggest ideas or report issues by [opening an issue](https://github.com/Nacho114/harpoon/issues/new).  
If you want to contribute code changes you will find some useful information in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

The content of this repository is licensed under the BSD-3-Clause license. See the [LICENSE](LICENSE) file for details.

## Acknowledgments

This plugin is based on the Zellij's [rust-example-plugin](https://github.com/zellij-org/rust-plugin-example).
