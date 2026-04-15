---
name: "fzf-binds"
description: "Default fzf key bindings reference. Use when configuring fzf keybinds, debugging conflicts with default bindings, or wiring fzf actions in the browser/selector."
---

# fzf Default Key Bindings

Reference list of fzf actions that have default key bindings out of the box. Use this when picking custom keybinds (to avoid clobbering defaults) or when wiring actions in `src/browser/selector.rs`.

## Regenerating This List

Run `fzf --man` and scroll to the `AVAILABLE ACTIONS:` section. Strip every action that has no default binding (entries with only a `(description)` and no key listed).

## Default Bindings

```
ACTION:                      DEFAULT BINDINGS (NOTES):
  abort                        ctrl-c  ctrl-g  ctrl-q  esc
  accept                       enter   double-click
  backward-char                ctrl-b  left
  backward-delete-char         ctrl-h  ctrl-bspace  bspace
  backward-kill-word           alt-bs
  backward-word                alt-b   shift-left
  beginning-of-line            ctrl-a  home
  clear-screen                 ctrl-l
  delete-char                  del
  delete-char/eof              ctrl-d  (same as delete-char except aborts fzf if query is empty)
  down                         ctrl-j  down
  down-match                   ctrl-n  alt-down  (move to the match below the cursor)
  end-of-line                  ctrl-e  end
  forward-char                 ctrl-f  right
  forward-word                 alt-f   shift-right
  kill-word                    alt-d
  next-history                 ctrl-n  (on --history)
  page-down                    pgdn
  page-up                      pgup
  prev-history                 ctrl-p  (on --history)
  preview-down                 shift-down
  preview-up                   shift-up
  toggle                       right-click
  toggle-wrap                  ctrl-/  alt-/
  toggle+down                  ctrl-i  (tab)
  toggle+up                    btab    (shift-tab)
  unix-line-discard            ctrl-u
  unix-word-rubout             ctrl-w
  up                           ctrl-k  up
  up-match                     ctrl-p  alt-up  (move to the match above the cursor)
  yank                         ctrl-y
```

## Notes

- `ctrl-n` / `ctrl-p` are overloaded: they map to `down-match` / `up-match` by default, but switch to `next-history` / `prev-history` when `--history` is set.
- `ctrl-i` is the same keycode as `tab`; `btab` is `shift-tab`.
- `toggle` is bound to mouse right-click, not a key.
- If binding a custom action to any key listed above, you are overriding the default — consider whether that's intended.
