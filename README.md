# zellij-leap

Jump to a Zellij tab by typing a few characters of its title

Inspired by [leap.nvim](https://codeberg.org/andyg/leap.nvim) and its predecessors

> [!TODO]
> Proper README structure

## Zellij config

```kdl
keybinds {
    normal {
        bind "Space" {
            LaunchOrFocusPlugin "file:/absolute/path/to/zellij-leap.wasm" {
                floating true
                move_to_focused_tab true
            }
        }
    }
}
```
