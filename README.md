# zellij-leap

Jump to a Zellij tab / pane / session by typing a few characters of its name

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
                // Defaults:
                leap_target "tab"
                leap_on_no_match "reset"
                leap_on_pane_unfocus "none"
                leap_on_escape "close"
                leap_suppressed_panes "include"
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
- `session` - Jump to any session (including resurrectable ones)
- `session_except_current` - Jump to any session (including resurrectable ones) except current one

## Special keys

Sometimes one character isn't enough to find a unique match - in that case you'll be prompted to type more characters. If the sequence is too long you can use manual selection

- `Esc` - Reset current matching, or close/hide the plugin if nothing is being matched (depending on `leap_on_escape`)
- `Up` / `Ctrl-k` / `Ctrl-p` - Move selection up manually. Useful when you have targets with very similar names
- `Down` / `Ctrl-j` / `Ctrl-n` - Move selection down
- `Enter` - Jump to the currently selected target (indicated by `»` or `>` sign)
- `Ctrl-u` - Reset current matching

## Options

- `leap_target` - see "Targets" section
- `leap_on_no_match` - behavior when no match found: `reset`, `close`, or `hide_floating_panes`
- `leap_on_pane_unfocus` - behavior when pane loses focus: `none` or `close`
- `leap_on_escape` - behavior of escape key: `close`, `reset_or_close` , `hide_floating_panes` or `reset_or_hide_floating_panes`
- `leap_suppressed_panes` - behavior for suppressed panes (e.g., stacked ones): `exclude` (don't show at all), `dont_match` (manual selection only), or `include`

## Matching algorithm

1. Search each tab name for occurrences of the typed character (case-insensitive)
2. Filter out tabs that don't contain the character
3. If the character appears multiple times, remember list of possible next characters and pick one on second key press. Second character must be unique, otherwise plugin picks leftmost match
4. For subsequent characters, search only the portion of the name after the previous match
5. Automatically jump to the tab when only one match remains
