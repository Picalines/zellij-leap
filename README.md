# zellij-leap

Jump to a Zellij tab by typing a few characters of its title

Inspired by [leap.nvim](https://codeberg.org/andyg/leap.nvim) and its predecessors

![demo](./assets/demo.cast.gif)

## Installation

1. Download the latest `zellij-leap.wasm` from the releases page
2. Add it to your Zellij plugin aliases

```kdl
plugins {
    about location="zellij:about"
    session-manager location="zellij:session-manager"
    // ...
    leap location="file:/absolute/path/to/zellij-leap.wasm"
}
```

## Configuration

```kdl
keybinds {
    normal {
        bind "Space" {
            LaunchOrFocusPlugin "leap" {
                floating true
                leap_target "tab"                    // Target. See "Targets" section below
                leap_on_pane_unfocus "none"          // Behavior when pane loses focus: "none" or "close"
                leap_on_escape "close"               // Behavior on escape key: "close" or "hide_floating_panes"
            }
            SwitchToMode "normal";
        }
    }
}
```

### Targets

- `tab` - Jump to any tab, including the currently active tab
- `tab_except_active` - Jump to any tab except the currently active one
- `pane_in_active_tab` - Jump to any pane within the currently active tab
- `session` (work in progress, might work buggy) - Jump to any session (including resurrectable ones)

## Matching algorithm

1. Search each tab name for the first occurrence of the typed character (case-insensitive)
2. Filter out tabs that don't contain the character
3. For subsequent characters, search only the portion of the name after the previous match
4. Automatically jump to the tab when only one match remains
