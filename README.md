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
- `session_except_current` - Jump to any session (including resurrectable ones) except the current one

## Matching rules

Sometimes one character isn't enough to find a unique match. In that case, you'll be prompted to type more characters:
- White letters are the ones you can type to narrow the match
- Green letters are the ones you have typed previously
- Dim letters are skipped in further matching

If the match has reached the end of the target, an underline will be shown. It means that the `Space` key can be used on it from now on

## Special keys

If the character sequence is too long, you can use manual selection

- `Esc` - Reset current matching, or close/hide the plugin if nothing is being matched (depending on `leap_on_escape`)
- `Tab` / `Shift-Tab` - Move the selection to the next / previous target still being matched
- `Up` / `Ctrl-k` / `Ctrl-p` - Move the selection up manually. Moves even to discarded targets. Useful when you have targets with very similar names
- `Down` / `Ctrl-j` / `Ctrl-n` - Move the selection down
- `Enter` - Jump to the currently selected target (indicated by the `»` or `>` sign) (even a discarded one)
- `Ctrl-u` - Reset current matching

## Options

- `leap_target` - see the "Targets" section
- `leap_on_no_match` - behavior when no match is found: `reset`, `close`, or `hide_floating_panes`
- `leap_on_pane_unfocus` - behavior when the pane loses focus: `none` or `close`
- `leap_on_escape` - behavior of the escape key: `close`, `reset_or_close` , `hide_floating_panes` or `reset_or_hide_floating_panes`
- `leap_suppressed_panes` - behavior for suppressed panes (e.g., stacked ones): `exclude` (don't show at all), `dont_match` (manual selection only), or `include`
