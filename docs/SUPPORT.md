# Support

Fixes for things people run into, by system, then the ones that are the same everywhere. If yours
isn't here, see [Reporting a problem](#reporting-a-problem) at the end.

- [Windows](#windows)
- [macOS](#macos)
- [Linux](#linux)
- [On every system](#on-every-system)
- [Where OctaMEDIC keeps its files](#where-octamedic-keeps-its-files)
- [Reporting a problem](#reporting-a-problem)

## Windows

### "Windows protected your PC"

OctaMEDIC isn't signed with a code signing certificate, so SmartScreen warns about it the first
time. Click **More info**, then **Run anyway**. It only asks once per download.

If there's no **Run anyway**, your PC's policy (often a work or school PC) blocks unsigned apps.
Right-click the zip *before* unzipping it, choose **Properties**, tick **Unblock** at the bottom and
click **OK**, then unzip it again. If it's still blocked, ask whoever manages the PC.

### My antivirus says it's a virus, or deletes it

Unsigned programs that nobody has downloaded much yet sometimes get flagged by mistake. The release
zips are built from the source on GitHub by the
[release workflow](https://github.com/TheExiledCat/octamedic/actions/workflows/release.yml), so you
can check the build that made yours. To run it anyway, restore it from your antivirus' quarantine
and add the folder as an exception (in Windows Security: **Virus & threat protection > Manage
settings > Exclusions**). You can also [build it yourself](../README.md#building-from-source).

### "VCRUNTIME140.dll was not found" or "MSVCP140.dll is missing"

The Windows builds need Microsoft's Visual C++ runtime, which most PCs already have. Install the
latest **Visual C++ Redistributable** from
[Microsoft](https://learn.microsoft.com/cpp/windows/latest-supported-vc-redist): `X64` for most
PCs, `ARM64` for Windows on ARM.

### The window stays white, or opens and closes straight away

Usually the graphics driver doesn't get on with the graphics backend OctaMEDIC picked. Try another
one: open a Command Prompt in the folder you unzipped (type `cmd` in Explorer's address bar and
press Enter) and run

```bat
set OCTAMEDIC_GRAPHICS=dx12
octamedic_gui.exe
```

If that doesn't help, try `vulkan`, then `gl`. When one works, put it in `config.toml`
so it's used every time:

```toml
graphics = "dx12"
```

([CONFIG.md](CONFIG.md#graphics) has the details.) It's also worth updating your graphics driver
from NVIDIA, AMD or Intel's site, and trying with overlays turned off (Steam, Discord, OBS,
RivaTuner/MSI Afterburner, GeForce Experience). Remote Desktop and virtual machines often don't
have the graphics support it needs.

#### Older graphics cards

Older graphics often have a Vulkan driver that's no longer updated, and OctaMEDIC picks Vulkan
when it's there, so `dx12` is the one to try first. If the card has no DirectX 12 either, try
`gl`, with the newest driver its maker still offers for it.

The oldest cards, with neither Vulkan nor DirectX 12 and an OpenGL older than 4.3, may not run it
at all: OctaMEDIC's graphics library needs features they don't have. Please still report it.

Please add your `crash.log`, graphics card and which backend worked to
[issue #53](https://github.com/TheExiledCat/octamedic/issues/53), so it can be fixed for everyone.

### It closes with no message and no crash log

A few kinds of crash end the program before it can write anything:

- **To see a stack overflow or other last words**, run it from a Command Prompt with its errors
  going to a file, then attach `err.txt` to your report:

  ```bat
  octamedic_gui.exe 2> err.txt
  ```

- **For a crash in a driver**, open **Event Viewer**, go to **Windows Logs > Application**, and
  look for an **Application Error** for `octamedic_gui.exe` at the time it closed. Its **Faulting
  module name** says what crashed: a name like `nvoglv64.dll` or `nvwgf2umx.dll` (NVIDIA),
  `atiumd64.dll` or `amdxx64.dll` (AMD), or `igd…` (Intel) is the graphics driver, and the
  backend tip above is worth trying.

### Which zip do I need?

`x86_64-pc-windows-msvc` for almost every PC. `aarch64-pc-windows-msvc` is only for Windows on ARM
(Snapdragon laptops, Surface Pro X). To check, open **Settings > System > About**: **System type**
says x64 or ARM-based.

## macOS

### "OctaMEDIC can't be opened" or "Apple could not verify…"

OctaMEDIC isn't notarized by Apple, which needs a paid developer account, so Gatekeeper stops it
the first time. To open it anyway:

1. Try to open it once, and click **Done** (or **OK**) on the warning.
2. Open **System Settings > Privacy & Security** and scroll down to **Security**. There's a line
   saying OctaMEDIC was blocked: click **Open Anyway** and confirm with your password or Touch ID.
3. Open it again and click **Open**.

On macOS 14 and older, right-clicking the app and choosing **Open** also works. macOS 15 removed
that, so use the steps above.

Or do it in Terminal, which also works when the button doesn't show up:

```sh
xattr -dr com.apple.quarantine /Applications/OctaMEDIC.app
```

### "OctaMEDIC is damaged and can't be opened. You should move it to the Trash"

It isn't damaged; macOS says this about some downloaded apps that aren't notarized. Run the
`xattr` command above, then open it again. If it still says so, download the zip again with Safari
and unzip it with Finder (double-click it), since some other unzip tools leave the app's signature
broken.

### The command line tool won't run

`mmd_cli` needs the same, once, in the folder you unzipped it to:

```sh
xattr -d com.apple.quarantine mmd_cli
chmod +x mmd_cli
./mmd_cli song.mmd1
```

### "You can't open the application because this Mac doesn't support it"

You have the wrong zip. `aarch64-apple-darwin` is for Apple Silicon (M1 and later), and
`x86_64-apple-darwin` for Intel Macs. **Apple menu > About This Mac** says which you have under
**Chip** (Apple M…) or **Processor** (Intel). The Intel build also runs on Apple Silicon through
Rosetta, but slower.

### Double-clicking a module doesn't open it

Not yet. Open it with LOAD SONG on the FILES panel, or from Terminal with OctaMEDIC closed:

```sh
open -a OctaMEDIC --args ~/Music/song.mmd1
```

Give the whole path, since the app doesn't start in the folder you're in.

### Which key is the Amiga key?

Cmd, or the right Option key. Cmd+Q quits, as usual.

## Linux

### "Permission denied" when running it

Some unzip tools don't keep the file executable:

```sh
chmod +x octamedic_gui
./octamedic_gui
```

### "error while loading shared libraries: libasound.so.2"

It needs ALSA's library, for sound. Install it with your package manager:

| Distro | Command |
|---|---|
| Debian, Ubuntu, Mint, Pop!_OS | `sudo apt install libasound2` (on Ubuntu 24.04 and later, `libasound2t64`) |
| Fedora | `sudo dnf install alsa-lib` |
| Arch, Manjaro | `sudo pacman -S alsa-lib` |
| openSUSE | `sudo zypper install libasound2` |

PipeWire and PulseAudio systems still need it: OctaMEDIC talks to them through ALSA.

### "version `GLIBC_2.xx' not found"

The Linux builds are made on Ubuntu 22.04, so they need glibc 2.35 or newer (Ubuntu 22.04,
Debian 12, Fedora 36 and later). On anything older,
[build it from source](../README.md#building-from-source).

### The window stays black or white, or doesn't open

Make sure your graphics drivers have Vulkan or OpenGL support (on Debian and Ubuntu, the
`mesa-vulkan-drivers` package for AMD and Intel, or NVIDIA's own driver), then try the other
backend:

```sh
OCTAMEDIC_GRAPHICS=gl ./octamedic_gui
OCTAMEDIC_GRAPHICS=vulkan ./octamedic_gui
```

On Wayland, if the window closes straight away, try it through XWayland:

```sh
WAYLAND_DISPLAY= ./octamedic_gui
```

### No message box when it crashes

The crash message uses `zenity` or `kdialog`. Without either installed, it's only written to
`crash.log`. Installing `zenity` gets you the message.

### In WSL

The GUI runs in WSL, but sound through WSL's audio is often crackly or distorted, and under WSLg's
Wayland the window closes straight away (run it with `WAYLAND_DISPLAY=` as above). The Windows
build works better on the same machine.

## On every system

### No sound

- If the notification bar says **NO AUDIO**, OctaMEDIC couldn't open a sound device. Check that one
  is plugged in and switched on, and that another program isn't holding it in exclusive mode
  (some DAWs and ASIO drivers do), then restart OctaMEDIC.
- Open VOL's **OUTPUT** and pick the device you're listening on. If the device you picked before
  is gone, OctaMEDIC uses the system's default and says so.
- Check the master volume and that the tracks aren't muted on the EDIT panel.

Ctrl+` opens the debug console, which lists everything that went wrong while starting up.

### My keyboard has no right Alt (the Amiga key)

The Windows key works as the Amiga key too, and Cmd on a Mac. On many European layouts the right
Alt is AltGr; if that doesn't act as the Amiga key, use the Windows key.

### A song I saved won't open in the real OctaMED

Songs are saved as MMD1 with their instruments, the format OctaMED Professional brought in, so
MED and the OctaMED versions before Professional can't read them. If OctaMED Pro v4 can't open it
either, that's a bug: please [report it](#reporting-a-problem) with the file.

### A module won't load

OctaMEDIC loads MMD0 and MMD1 modules, whatever the file is called: it goes by what's inside.
MMD2 and MMD3 (OctaMED Soundstudio), ProTracker and SoundTracker modules can't be loaded yet. If an
MMD0 or MMD1 module won't load, please report it with the file.

### My settings are gone, or a setting is broken

If `config.toml` can't be read, OctaMEDIC renames it to `config.toml.bak`, starts with the defaults
and says so in a notification. To start over on purpose, close OctaMEDIC and delete `config.toml`.
[CONFIG.md](CONFIG.md) has every setting.

### The screen looks blurry, curved or noisy

Those are the screen filters. Ctrl+F11 opens them, with **Toggle all** to switch them all off.

## Where OctaMEDIC keeps its files

| System | Folder |
|---|---|
| Windows | `%LOCALAPPDATA%\octamedic` (paste it into Explorer's address bar) |
| macOS | `~/Library/Application Support/octamedic` (in Finder: **Go > Go to Folder…**) |
| Linux | `~/.config/octamedic` |

`config.toml` holds the settings, and `crash.log` the last crash. When OctaMEDIC crashes, a message
says so and its **Open crash log** button (**Yes** on Windows) opens it.

## Reporting a problem

[Open an issue](https://github.com/TheExiledCat/octamedic/issues/new/choose) with the bug report
template, or ask on the Discord linked from the [README](../README.md). It helps to include:

- Your system and version (e.g. Windows 11 24H2, macOS 15.1, Ubuntu 24.04), and which zip you
  downloaded
- What you did, what happened, and what you expected (or what the real OctaMED does)
- `crash.log`, if there is one, or the output from the tips above if there isn't
- Your graphics card, for anything to do with the window
- The module, if it's about a song (please no copyrighted samples)
