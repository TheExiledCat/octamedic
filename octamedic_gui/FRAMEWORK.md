# UI framework

The GUI is built on [rustuition](../rustuition/README.md), the Amiga style GUI library in this
workspace; read that first for gadgets, layout, drawing, fonts and buttons. This is what the app
adds on top, in `src/framework/`, and how OctaMED's screens in `src/app/` are put together.

## What the app adds

- `framework/event.rs`: the app's `Event`, rustuition's mouse events plus `Wheel`, `Key` and
  `Text` from ggez. It implements `PointerEvent`, so rustuition's gadgets take it as is. `Wheel`
  is ggez's raw delta, left for each handler to interpret.
- `framework/wheel.rs`: `WheelLines`, owned by handlers that scroll by lines. Wheel delta to whole
  lines, capped at one line per event since ggez doesn't separate wheel and trackpad deltas.
- `framework/widgets/mod.rs`: `Button` and `Slider` aliases with `AppState` and `Event` filled in.
  Use these rather than rustuition's, so handlers know they get an `AppState`. App gadgets
  implement `Gadget<AppState, Event>`. Make text boxes with `widgets::textbox()`, which draws the
  cursor and selection in OctaMED's pink with the text over them mixed in, rather than
  `Textbox::new()`.
- `framework/widgets/symbols.rs`: OctaMED's picture button contents (`blocks`, `MusicSymbol`,
  `stacked_text`), and `audio_view.rs` the scopes and VU meters.
- `app/fonts.rs`: OctaMED's own fonts, redrawn by hand in `assets/fonts/` since the originals
  aren't around anywhere. `INTERFACE` (OctaMED's own 8x8, for numbers and names like the status
  row: its 1, 2, 4, 7, - and = differ from Topaz, which it falls back on for the rest), `PANEL` (gadget labels like `FILES`, 6 lines, capitals
  only), `SMALL` (the status row flags like `CHRD`, 5 lines), `LABEL` (play panel labels), `TITLE`,
  `AUTHOR`, `EDITOR` (16px wide block editor chars), `TRACKS` (the track digits of TRK ON: and
  SELECT:), `TRACKS_BOLD` (TRKS:'s, which OctaMED drew bolder and wider rather than just bold),
  `TALL` (SET and CLR), `BIG` (the EDIT panel's OCTAVE digits, EDIT ON and SPC, and RANGE's
  CHORD), `TIMER_LABEL` (`BIG` with the title bar's own M and R) and `LABEL_SMALL_DOT` (`LABEL`
  with the 1px full stop of L.ALT and R.ALT). Letters and
  digits always come from a font; only symbols like arrows are pictures. `cargo test` checks the
  files parse.
- `app/pens.rs`: OctaMED's pens past rustuition's four, `RED`, `ROSE`, `PINK` and `BLUSH`. The
  palette is the song's, turned into the screen's with `state::screen_palette`.
- `app/mod.rs`: `SCREEN`, PAL hires.
- `app/icon.rs`: the window's icon, `assets/icon.png`, built into the binary. It shows on Windows
  (title bar and taskbar) and X11. macOS takes the Dock icon from an app bundle and Wayland from a
  desktop file, and the release doesn't make either yet.

Keep the sizes measured from OctaMED as named constants where they're used, like the status row's
`WIDTHS`.

## Keys

Keys go through a `Keymap` of actions (`keymap.rs`) so they can be rebound by action id later:

```rust
#[derive(Clone, Copy, PartialEq, Debug)]
enum EditorAction { Up, Down }
impl Action for EditorAction {}

let keys = Keymap::new("editor")
    .bind(Key::plain(KeyCode::Up), EditorAction::Up, "Up")
    .bind(Key::plain(KeyCode::Down), EditorAction::Down, "Down");
```

Implement `KeyActions` (`keymap`, `perform`, `describe`) on the gadget and call
`self.dispatch(state, event)` from `handle_event`. Ids are `scope.action`, e.g. `editor.up`.

## The app

`MainView` (`app/main_view.rs`) is the whole screen and handles global keys like Ctrl+Q, and last of
all Space (stop) and Shift+Space (continue the song), so a text field gets them first. The
keyboard's note keys work the same way: any screen plays them on the cursor's track, unless a text
field or the block editor took them first. Every panel sees every mouse click, and within a panel
every text box does (`|` rather than `||`), so a box finds out when a click on another one takes its
focus; anything else stops at the first that takes it. The other play keys are OctaMED's: Amiga+P
and Shift+Alt+Space play the song, Amiga+Space the block, Alt+Space continues the block. The Amiga
key is right Alt, where it was on the real keyboard next to right Ctrl; the Windows key (Cmd on a
Mac) counts too. `App` tells the two Alts apart itself (`keymap::amiga_mods`), since the modifiers
don't, and shortcuts use `Key::amiga`. Ctrl and the keypad edit the play sequence like OctaMED's: 7
and 1 go to the top and end, 8 and 2 move up and down, 4 and 6 change the block number, 5 inserts
the current block, 0 inserts block 0 and . deletes. Inside it: `TitleBar` (timer, title, author),
`CopyrightBar` (the screen title over the title bar, toggled by a right click nothing else takes,
its gadget quits), `Header` (`app/header/mod.rs`), `ModeButtons`, `StatusBar` (song position, block,
instrument, song), the `BlockEditor` (`app/main/block_editor.rs`, with track numbers and the block's
`TrackCount` over its top rows), `Footer` (info line and scopes) and `DebugConsole` (toggle with
Ctrl+`). Most live in a file named after them in `app/`.

The info line's memory numbers are the project's, not the PC's: first what has to fit in an
Amiga's chip memory (sample data and synth waveforms, `OctamedicProject::chip_bytes`), then
about what the project takes saved as an MMD1 module (`module_bytes`). Both are in
`octamedic_core/src/data/memory.rs`.

The screen is scaled to fit the window (`ScaleMode::Fit`). winit can't hold a window to an aspect
ratio while it's dragged, so `WindowShape` (`app/window_shape.rs`) puts a windowed window back in
the screen's shape once resizing stops: the side that was dragged keeps its size and the other
fits to it (dragging a corner, the one that changed more wins), staying on the monitor; maximised and fullscreen windows are left alone, with bars at the sides.

Ctrl+F11 opens the screen filters (`app/screen_filters.rs`), a list of rustuition's filters in the
order they run. While any is on, the screen and the pointer are painted into a `Framebuffer`
instead of straight onto the GPU, filtered, and uploaded as the screen image.

The VOL page's OUTPUT button opens the output dialog (`app/output.rs`): which sound device plays
(cpal's output devices, or the default) and whether it's mixed to mono. OK on another device has
`AppState::set_output_device` close the engine and start it again on that one with
`AppState::open_audio`, which sends it the song, mutes and mono; that stops playing. If the new
device can't be opened it goes back to the old one, and the notification bar says why. If the
engine has no device at all (it couldn't open one at startup, say), OK tries again even on the same
device.

The output device, mono and PAL/NTSC are `Config` (`app/config.rs`), saved in `config.toml` (see
[docs/FILES.md](../docs/FILES.md)). `main` loads it before the window opens, so the window and the
engine start out on the saved video standard and device. `AppState` keeps it private: read it with
`config()` and change it with `config_mut()`, which marks it changed, and `AppState::update` saves
it at the end of that frame, so nothing else has to remember to. If the saved device can't be
opened at startup the engine runs on the default one for that run (`using_fallback`), the saved
setting stays for when the device comes back, and the startup errors say so.

The mode buttons set two modes: `HeaderMode` (which panel the header shows) and `MainMode` (what
fills the screen below). Each button's `ModeBinding` in `app/mode/buttons.rs` says which. Header
panels are in `app/header/`, one file each (`misc.rs` is MISC), and `HeaderPanels::panel` maps
each `HeaderMode` to one. A panel draws inside the header's sunken area and gets the header's
updates and events while it's showing. Main screens are in `app/main/` and `MainScreens::screen`
does the same for `MainMode`. To add one: write the gadget, add a field for it, return it from
`panel` or `screen`.

FILES, PLAY, VOL and MISC's palette work. PLAY's HEX and DEC say how volume commands are read, and
with Shift held they also rewrite the song's volume commands so they play the same
(`AppState::convert_volumes_hex`). INSTR (`header/instrument.rs`) shows the active instrument and
edits its name, transpose, finetune, volume, hold and decay (`AppState::edit_active_instrument`,
hold and decay take two digits, 0 to 99); its arrows go to the first, previous and next instrument
and the last one with a sound (instrument 1 if none has one), and FLUSH empties it: no sample, no
name, and every value 0, volume included. Changing the active instrument lets go of any of its boxes
being typed in, so what's typed next can't land in the new instrument. The rest of it is ghosted.
The rest of MISC and the SLIST screen (`main/song_list.rs`) are laid out like OctaMED with their
buttons ghosted, ready to be hooked up. `main/list_scroller.rs` is the slider with arrows that the
FILES and SLIST lists share. The wheel scrolls it too, with the pointer over the list or the
slider, through its own `WheelLines`, so whoever draws the list sets its `list_area` as well as
`lines_that_fit`.

The sample editor (`main/sample_editor.rs`) is laid out like OctaMED's SMPED. BUFFSIZE is the whole
sample's length and the number at the waveform's top right the length in view. `WaveView`
(`main/wave_view.rs`) is what's in view: each zoom level halves it around its middle, ZOOM OUT
doubles it again and SHOW ALL goes back to the whole sample, as does picking another instrument.
SHOW RANGE shows just the range, and zooming out from there doubles it until it's the whole sample.
Left and Right move the view half its length either way, stopping at the sample's ends. The
waveform, its zero line and the scroll bar mix their pens where they cross, the way OctaMED's
bitplanes do. The waveform takes one sample a column, joined to the one before with a line, or on
its own with D.PIXELS on.

The range is dragged out on the waveform with the right button, and shift stretches the one there
is to the sample clicked. `WaveformMouse` (`main/waveform_mouse.rs`) follows a press on the
waveform until it's let go. The range's columns are highlighted, RNGSTART and RNGEND show it and
can be typed in, RANGE ALL makes it the whole sample, CLEAR silences it and DEL cuts it out.
The copy buffer is `state.clipboard.sample`: COPY puts the range in it, CUT does that and then what DEL
does, PASTE puts it in at the range's start, S>CBUFF copies the whole sample and CBUFF>S swaps the
sample for it. MIX mixes the buffer in from the range's start, only as far as the range goes if
there is one, and REV turns the range round. BUFFSIZE takes a new length after Enter, with a prompt to clear or keep the sample.

LOOP ON switches the sample's loop (`OctamedicSample::repeat`) on and off. Turned on it's the
whole sample, or what it was when it was last turned off if the sample hasn't changed since; a
multi-octave sample's loop stays in its
shortest copy (`loop_limit`). While it's on, the loop's ends show as OctaMED's sprite markers: a
blue line with a flag, the start's flag to its left and the end's to its right, the selected one's
filled yellow. S and E pick which end the arrows move: < and > by 2 bytes, <0 and 0> to the next
silent sample on an even byte. Dragging a marker's flag or line moves it as far as the mouse goes
and selects it, and only shown markers can be grabbed. MMD stores a loop in words, takes a one word
loop for none and its length has to fit a word, so every loop goes through
`instrument::word_loop`: even ends, 4 bytes to 128K long.

PITCH is the period the sample sounds right at and DEST the one CHANGE RATE moves it to; the
boxes next to them show its note, or --- if it isn't one of the table's (`period::period_note`),
and clicking one then pressing a note key sets its period to that note's. CHANGE RATE resamples
the sample so that played at PITCH it sounds like DEST did (`OctamedicSample::resampled`, the loop
stretched with it), and OCT ▲ and OCT ▼ halve and double it. Anything that would come out longer
than BUFFSIZE's 131072 is turned down. Shortening averages the samples each
new one covers and lengthening draws the ones between on a line, so both smooth the sound like
OctaMED's; up and down an octave a few times comes back noticeably duller.
PLAY DISPLAY and PLAY RANGE play at the note nearest PITCH.
The rest of the buttons are ghosted.

Change project data through `AppState` methods like `enter_note` and `edit_track`, not directly.
They track which blocks changed and send them to the engine in `AppState::update`, so you hear
edits while it's playing.

`state.debug("...")` adds a line to the debug console.

On Windows the GUI is a GUI program (`windows_subsystem = "windows"`), so no console opens with it
and nothing printed shows anywhere. Errors from before the window opens (the folders, a module
given on the command line, no audio) are collected in `main` and `App::new` and handed to
`AppState::report_startup_errors`: each goes in the debug console (Ctrl+`), and the notification
bar shows the error, or "Multiple errors have occurred" if there's more than one. Errors that stop
the window opening at all end `main`, and they go in `crash.log` in the config folder (the temp
folder if there isn't one) with the startup errors before them. Panics go there too, with a
backtrace (`crash_log.rs`). Only the last crash is kept.

Dialogs are rustuition's `Dialog` (the `Dialog` alias in `framework/widgets`). Open one with
`state.open_dialog(dialog)`, from anywhere that has the state, even another dialog's callback.
`DialogStack` (`framework/dialog_stack.rs`, owned by `App`) picks them up, gives the top one every
event and draws them over everything.

A prompt (`app/prompt.rs`) is the small version: a line of text in the notification bar, opened
with `state.open_prompt(prompt)`. Until a key's pressed it gets every event and nothing else does;
the first key that isn't a modifier closes it and goes to its closure with the state.
`Prompt::yes_no` is the usual "(Y/N)" question. `state.notify(text)` shows a message in the same
bar for a few seconds. The bar is the title bar's height and covers it.

A shortcut for something the status bar shows goes in the status bar's keymap
(`app/status_bar.rs`), and since the status bar is on every screen, so are they: F1 to F5 for the
octaves, Esc for edit mode (E), ~ for SP, Shift+Up/Down and Alt+Up/Down for the previous, next,
first and last block, the instrument keys, and Ctrl and the keypad (or the keypad with Caps Lock
on) for the play sequence. The header comes before the screen below it, so a screen with text boxes
sets `AppState::typing` in its update while one has the keyboard, and the status bar leaves keys
alone then (the sample editor's boxes do).

The status bar's box next to E shows the keyboard's two octaves, the bottom row's and the top
row's, in hex like OctaMED: 12 up to 9A. Clicking it goes up (right click down) and loops round;
F1 to F5 only go up to 5. Shift+Left and Shift+Right change the instrument. Holding a note key
repeats it like a new press.

In edit mode the block editor types into whichever column the cursor is on: a note key in the
note column, a base 32 digit (0-9, A-V) in the instrument's right column, and a hex digit in the
command's and the value's. Each moves the cursor down a line, like a note. Like OctaMED,
the cursor skips the instrument's left digit, so instruments past 0V go in by entering a note with
the instrument selected.

SP in the status bar (or the ~ key) switches spacing on: then entering anything and the up and down
arrows move by the line add (`AppState::line_step`) instead of one line. The line add is never below
1.

With Caps Lock on the keypad edits the play sequence without Ctrl. Without it the keypad does what
the EDIT panel's KEYPAD and MAP2 pick (`app/keypad.rs`): TRK ON/OFF turns tracks on and off,
SEL.INSTR picks the first 16 instruments, and MAP2 swaps in OctaMED's second layout of each (digits
for tracks with `.` all off and Enter all on, or digits within the instrument's ten with `.` ten on,
`+` and `-` next and previous and `*` the nearest instrument above the cursor). The Amiga keypad's
`(` and `)` have no key on a PC's, so what they do is left out. A keypad key never types a note:
`App` drops the character it types unless a box is being typed in. The modifiers don't carry Caps
Lock, so `App` flips `AppState::caps_lock` on the key and corrects it from typed letters. Alt and
the keypad go through the instrument list like OctaMED's (7 and 1 first and last, 8 and 2 one up and
down, 9 and 3 a page of 8), Alt+Shift+Left/Right jump 16, Amiga+I opens INSTR with its name box
selected and Amiga+Shift+I clears it first. Amiga+F switches the filter. In the command and value
columns Alt and a number key types 10 more, so Alt+0 to Alt+5 are A to F.

The EDIT panel (`header/edit.rs`) has TRK ON 0 to F and SET/CLR for the mutes, KEYPAD, OCTAVE,
EDIT ON, SPC with the line add, and SPACE=DEL, which makes Space delete like Del instead of
stopping (Shift+Space still continues). ADV. picks where the cursor goes after a note, a hold or
Del in the note column, up or down by the line add or a track left or right
(`AppState::advance_after_note`), and the two command arrows, which can be switched off again,
move it a digit right or left after a command digit instead (`advance_after_digit`). With the
loudspeaker on, moving the cursor up and down plays the notes on the line it lands on. The
programmable keys (CLR, the key and its definition, L.ALT and R.ALT, PICK:) are drawn but ghosted.
Its lettering differs from the other panels' a pixel here and there, so some of it has fonts of its
own (`TALL`, `BIG`) and the rest is placed letter by letter.

In edit mode Del clears the note and its instrument, on the instrument column just the
instrument, and on a command or value column just the digit under the cursor, and Shift+Del the whole cell; both move down like entering a note.
Backspace takes the cell out of its track, the rest of the track moving up into the gap, and
Shift+Backspace opens an empty one, pushing the rest down; the block keeps its length
(`OctamedicPattern::pull_track_up` and `push_track_down`). Alt+Backspace and Shift+Alt+Backspace are
the BLOCK panel's DEL.TRK and INS.TRK, and like the buttons don't need edit mode.

Amiga+O is OctaMED's SLIDE: VOL. With the cursor between two volume commands in a track (or on one
of them), it fills the lines between with volume commands going evenly from the first's volume to
the second's, rounded, read as hex or decimal like VOL says (`OctamedicPattern::slide_volume`). The
RANGE panel's SLIDE: VOL does the same.

Ctrl+Z undoes a change to the blocks and Ctrl+Y or Ctrl+Shift+Z redoes it (`app/history.rs`). OctaMED
has no undo, and its Ctrl keys don't use Z or Y.
Nothing calls into the history when it edits: once a frame, before the changes go to the engine,
`History::record` compares each block with the one it kept from the last frame and keeps the cells
that differ. Blocks are `Arc`s the engine holds too, so the old one costs nothing extra to keep
around, and a block that wasn't touched is skipped by pointer. Everything changed in one frame is
one step, so CUT is a single undo. A step holds the other version of its cells, which makes undo
and redo the same swap; a highlight keeps just its line, and a change of size keeps the whole old
block. Block names aren't undone: `AppState::edit_pattern_untracked` makes an edit the history
takes as it is, without a step. Undo goes to the block and puts the cursor on the first cell it changed, unless
it's playing, when the editor follows the engine's block. The history is capped at 1 MiB, dropping
the oldest steps. Undoing back to where the song was loaded or saved makes it saved again:
`is_saved` compares `History::position` with the one `mark_saved` kept, and block edits don't set
`has_changes`, which is left for the changes undo can't take back. Steps know their block by number, so the
history starts over when the number of blocks changes, a song is loaded or cleared, and when
Shift-clicking HEX or DEC rewrites the volume commands.

CHRD on the status bar and the RANGE panel's CHORD switch the same chord mode (`app/chord.rs`).
While it's on, the note keys held down together go on the cursor's track and then the tracks right
of it picked with SELECT: 0 to F, one note each; notes past the last of them are dropped. In edit
mode they're entered on the cursor's line and the cursor waits on the next chord track, and out of
it they're only played, on those tracks so they sound together. A key's repeats aren't more notes.
`App` ends the chord once every key but the modifiers is up (ggez's key ups don't reach the
gadgets), and with RST on the cursor then goes back to the chord's first track and down by the line
add. Shift+Return enters holds on every track of the last chord.

A range is a rectangle of cells in a block (`BlockRange` in `app/block_range.rs`, kept in
`AppState::range` with the block it's in). Dragging with the right button over the tracks makes
one, from any panel; it follows the mouse past the tracks and the block's ends, held to the nearest
cell. App keeps `AppState::right_button_held`, so a drag let go where the editor doesn't hear it
still ends. The range knows its block by number, so it's dropped when blocks are inserted,
deleted, split or joined, or another song is loaded, and it's kept inside its block if the block
gets smaller. Its cells are filled red, which turns the text over them rose, like OctaMED's
bitplanes. A
right click anywhere else still switches the displays off. The RANGE panel (`header/range.rs`)
works on it: SELECT TRACK and SELECT BLOCK make the cursor's track or the whole block the range,
CUT and COPY put it in `Clipboard::range` (its own, apart from the track and block buffers;
DEL BUFF clears all three), PASTE puts the buffer in from the cursor right and down, over what's
there, and PASTE SELECT does the same from the first track picked with SELECT: 0 to F rather than
the cursor's (SET picks them all, CLR none; the TRANS panel's SEL.TRKS uses them all). CLEAR
empties the range and CANCEL drops it. The
RANGE: boxes show its first and last track and line. SPREAD, SLIDE: 1 and 2 and NOTE ECHO are
ghosted. CHORD's label is embossed, shine over a shadow (`labels::embossed`), and RST is a
small button inside its right end. With no range the RANGE: boxes show `-` and `---`. The SELECT:
digits are `fonts::TRACKS` and SET and CLR `fonts::TALL`, shared with the EDIT panel (`header/labels.rs` has the hand placed labels the panels share).

The TRANS panel (`header/transpose.rs`, the edits in `app/transpose.rs`) works on the cells AFFECT
picks: every block of the song, the block, the cursor's track, the tracks picked with SELECT: 0 to F
or the range, and with CURR only those with the current instrument. 1/2 and OCT move their notes a
half step or an octave, leaving any that would go past C-1 or note 7F. Like OctaMED, a NOTE box
takes a note key only while it's held down; -> changes every first note to the second and <-> swaps
them. The instrument boxes take the current instrument when clicked, and DEL clears the first one's
notes, leaving their commands. All of it is one undo step. PLAY TRANSPOSE is the song's
`global_transpose`, which only the engine applies, and TEXT is the project's annotation, saved with
the module and shown on the notification bar when the song's loaded (INFO isn't hooked up, so it's
always saved). OctaMED's interface font draws `-` 6px wide with a gap either side, so
`assets/fonts/interface.txt` has it.

Enter or A in the note column puts in a hold: no note, just the selected instrument, shown as `-|-`
like any cell with an instrument and no note. It keeps a note held past its instrument's hold length
(see [docs/PLAYER.md](../docs/PLAYER.md)).

Tab highlights the cursor line, or takes the highlight off; it's saved in MMD1 modules. OctaMED
highlights by flipping a bitplane under the line, so every pen there swaps for its neighbour
(black and light grey, pink and blush, the band's greys); `highlight` in the block editor does
the same.

`app/files/mod.rs` knows the folders: the config folder (`config.toml`, `keybinds.toml`) and the
projects folder inside the music folder. `initialize` creates them at startup.

The FILES screen is `FileBrowser` (`app/main/file_browser.rs`): the current folder on the left
(`(dir) / Parent Dir`, then `(dir) name` for each folder, then the files with their sizes), and
shortcuts on the right (AmigaOS showed its assigns and devices there; we show the usual folders like
Music and Documents, then the drives, from `files::get_shortcuts`). Each list has a scroller, a
slider with up/down arrows. Where it is lives in `AppState::browser` (`app/files/browser.rs`), since
the FILES header panel (`app/header/files.rs`) uses it too: its DIR box shows the current path, and
opens what's typed when it loses focus, going back to the old path if that can't be read. The lists
scroll with their arrows and sliders, and go back to the top for a new folder. Clicking a folder,
the parent entry or a shortcut opens it, and clicking a file selects it, which FILE shows (typing in
FILE selects that name). LOAD SONG loads the selected file with `files::load_module` and
`AppState::load_project`, asking first if the song has unsaved changes. LOAD INSTR loads it into the
active instrument (`app/files/load_instrument.rs`, see [docs/FILES.md](../docs/FILES.md)), and DEL
FILE deletes it after asking. Buttons that don't do anything yet are disabled, so they're ghosted
like on the Amiga.

## Tests

Gadgets only need a window to draw, so you can test their logic on a state with no audio device:

```rust
let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
```

Set the gadget's area by hand if the test needs one (the button tests do), then send it events.
