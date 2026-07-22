# layout-switcher

[![CI](https://github.com/gimgiwer/layout-switcher/actions/workflows/ci.yml/badge.svg)](https://github.com/gimgiwer/layout-switcher/actions/workflows/ci.yml)

Исправляет текст, набранный не в той раскладке (`ghbdtn` → `привет`): демон по сигналу перепечатывает выделение в нужной раскладке. Встроено 8 раскладок: `en`, `ru`, `uk`, `de`, `by`, `kz`, `dvorak`, `colemak`. Поддерживаются свои раскладки и циклическое переключение целей.

## Сигналы

- `SIGUSR1` — читает выделение (`wl-paste -p`), определяет исходную раскладку, перепечатывает в целевой.
- `SIGUSR2` — то же + схлопывает повторяющиеся пустые строки (PDF).

### Выбор раскладки

1. **Источник**
   - Буквы-маркеры: `і` → `uk`, `ў` → `by`, `ә`/`ң`/`ғ` → `kz`, `ß` → `de`, кириллица без маркеров → `ru`. Один маркер — немедленный матч, `min_letter_hits` не применяется.
   - Без маркеров побеждает раскладка с максимальным покрытием букв; при равенстве — первая в конфиге (`min_letter_hits` действует).
2. **Цель**
   - Вторичная → `primary` (по умолчанию `en`).
   - `primary` → первая активная вторичная.
3. **Double Press**
   - Повторное нажатие в течение 2 секунд: демон распознаёт собственный результат по выделению, восстанавливает оригинал и переводит в следующую раскладку списка. Цикл `EN -> RU -> DE -> EN`, без IPC с композитором.

## Зависимости

- `wl-paste` / `wl-copy` (wl-clipboard)
- `wtype`

Только Wayland.

## Установка

### AUR

```sh
paru -S layout-switcher
systemctl --user enable --now layout-switcher
```

### Сборка

```sh
git clone https://github.com/gimgiwer/layout-switcher
cd layout-switcher
cargo build --release
cp target/release/layout-switcher ~/.local/bin/
cp layout-switcher.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now layout-switcher
```

## Бинды (niri)

```kdl
binds {
    Mod+Shift+X { spawn "pkill" "-USR1" "layout-switcher"; }
    Mod+Ctrl+Shift+X { spawn "pkill" "-USR2" "layout-switcher"; }
}
```

## Конфиг

Автогенерируется: `~/.config/layout-switcher/config.toml`

```toml
# secondaries get translated into this one
primary = "en"

# order matters: first one wins detection ties
layouts = ["en", "ru", "de"]

[tuning]
# fewer letters = too noisy to detect reliably
min_letter_hits = 3
# 1 MB cap so a fat selection can't stall the daemon
max_selection_bytes = 1048576
# clipboard fallback needs a beat before paste lands
clipboard_delay_ms = 150
```

### Свои раскладки

`~/.config/layout-switcher/layouts/my_layout.toml`:

```toml
name = "my_layout"
# 94 chars mapped to physical QWERTY keys
keys = "..."
```

Примеры — `layouts/` в репозитории.

## Лицензия

GPL-3.0-or-later

---

Fixes text typed in the wrong layout (`ghbdtn` → `привет`): the daemon retypes the selection in the correct layout on signal. 8 built-in layouts: `en`, `ru`, `uk`, `de`, `by`, `kz`, `dvorak`, `colemak`. Custom layouts and target cycling supported.

## Signals

- `SIGUSR1` — reads the selection (`wl-paste -p`), detects the source layout, retypes in the target one.
- `SIGUSR2` — same + collapses repeated empty lines (PDF).

### Layout selection

1. **Source**
   - Marker letters: `і` → `uk`, `ў` → `by`, `ә`/`ң`/`ғ` → `kz`, `ß` → `de`, Cyrillic without markers → `ru`. One marker is an immediate match; `min_letter_hits` is skipped.
   - Without markers, highest letter coverage wins; ties go to the first layout in the config (`min_letter_hits` applies).
2. **Target**
   - Secondary → `primary` (defaults to `en`).
   - `primary` → first active secondary.
3. **Double Press**
   - Repeat the hotkey within 2 seconds: the daemon recognizes its own output in the selection, restores the original, and translates to the next layout in the list. `EN -> RU -> DE -> EN` cycle, no compositor IPC.

## Dependencies

- `wl-paste` / `wl-copy` (wl-clipboard)
- `wtype`

Wayland only.

## Install

### AUR

```sh
paru -S layout-switcher
systemctl --user enable --now layout-switcher
```

### Build

```sh
git clone https://github.com/gimgiwer/layout-switcher
cd layout-switcher
cargo build --release
cp target/release/layout-switcher ~/.local/bin/
cp layout-switcher.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now layout-switcher
```

## Binds (niri)

```kdl
binds {
    Mod+Shift+X { spawn "pkill" "-USR1" "layout-switcher"; }
    Mod+Ctrl+Shift+X { spawn "pkill" "-USR2" "layout-switcher"; }
}
```

## Config

Auto-generated at `~/.config/layout-switcher/config.toml`:

```toml
# secondaries get translated into this one
primary = "en"

# order matters: first one wins detection ties
layouts = ["en", "ru", "de"]

[tuning]
# fewer letters = too noisy to detect reliably
min_letter_hits = 3
# 1 MB cap so a fat selection can't stall the daemon
max_selection_bytes = 1048576
# clipboard fallback needs a beat before paste lands
clipboard_delay_ms = 150
```

### Custom layouts

`~/.config/layout-switcher/layouts/my_layout.toml`:

```toml
name = "my_layout"
# 94 chars mapped to physical QWERTY keys
keys = "..."
```

See the repo's `layouts/` directory for examples — all eight built-ins are there.

## License

GPL-3.0-or-later
