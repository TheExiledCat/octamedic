# Configuration

OctaMEDIC keeps the settings that aren't part of a song in `config.toml`. It's written with the
defaults the first time the app starts, and saved again whenever you change a setting in the app,
so you rarely need to edit it by hand. Close the app before you do, or it may write over your
changes.

## Where it is

| System | Folder |
|---|---|
| Windows | `%LOCALAPPDATA%\octamedic` |
| Linux | `~/.config/octamedic` |
| macOS | `~/Library/Application Support/octamedic` |

`crash.log` is in the same folder. It holds the last crash, if there was one, and is worth
attaching to a bug report. When the app crashes, a message says so and has a button that opens it.

## An example

```toml
video = "pal"
graphics = "auto"

[output]
device = "alsa:pulse"
mono = false

[[filters]]
kind = "curvature"
enabled = true
amount = 0.04

[[filters]]
kind = "vignette"
enabled = false
strength = 0.6
size = 0.3
roundness = 0.4
```

Anything left out takes its default. If the file can't be read, or has a value that makes no sense
(like `video = "secam"`), it's renamed to `config.toml.bak` so nothing is lost, the app starts with
the defaults, and a notification says what was wrong.

## Settings

### `video`

`"pal"` (the default) or `"ntsc"`: the Amiga's video standard. It sets the screen's height and the
clock the player runs on, so songs play at the speed they would on that machine. Ctrl+F12 switches
between them in the app.

### `graphics`

What the screen is drawn with. `"auto"` (the default) tries Vulkan, Metal or DirectX 12 first,
then OpenGL. If the window stays white or closes as soon as it opens, try another:

| Value | Works on |
|---|---|
| `"dx12"` | Windows |
| `"vulkan"` | Windows and Linux, with a Vulkan driver |
| `"gl"` | Windows and Linux |
| `"metal"` | macOS |

The `OCTAMEDIC_GRAPHICS` environment variable takes the same values and wins over the file, to try
one without editing it. On Windows, from a Command Prompt in the folder you unzipped:

```bat
set OCTAMEDIC_GRAPHICS=dx12
octamedic_gui.exe
```

The crash log says which backend was asked for and which graphics card and driver were picked.

### `[output]`

Set in the app with VOL's OUTPUT button.

| Key | Default | What it does |
|---|---|---|
| `device` | left out | The sound device to play through, by its system id. Leave it out for the system's default device. |
| `mono` | `false` | Mixes the four channels to mono instead of the Amiga's hard left and right. |

### `[[filters]]`

The screen filters, set in the app with Ctrl+F11, in the order they run. Each `[[filters]]` entry
is one filter:

| Key | Default | What it does |
|---|---|---|
| `kind` | | Which filter, from the table below. Entries with a kind OctaMEDIC doesn't know are skipped. |
| `enabled` | `true` | Whether it's switched on. |
| settings | see below | The filter's settings, by the names in its Edit dialog in lowercase. Missing ones take their default, out of range ones are clamped. |

| `kind` | Settings (range, default) |
|---|---|
| `scanlines` | `strength` (0 to 1, 0.3), `spacing` (1 to 8 lines, 2) |
| `chroma` | `offset` (0 to 8 pixels, 1) |
| `vhs_wobble` | `amount` (0 to 8 pixels, 1.5), `wavelength` (4 to 256 lines, 64), `speed` (0 to 4, 0.5), `jitter` (0 to 4, 0.5) |
| `color` | `hue` (-180 to 180 degrees, 0), `saturation` (0 to 2, 1), `brightness` (0 to 2, 1), `contrast` (0 to 2, 1), `warmth` (-1 to 1, 0) |
| `curvature` | `amount` (0 to 0.25, 0.04) |
| `vignette` | `strength` (0 to 1, 0.6), `size` (0 to 0.9, 0.3), `roundness` (0 to 1, 0.4) |
| `noise` | `amount` (0 to 0.5, 0.08) |

Filters before `curvature` in the list work on the screen's own pixels, and filters after it work
on the finer pixels curvature is given, so for example scanlines placed before it bend with the
tube.
