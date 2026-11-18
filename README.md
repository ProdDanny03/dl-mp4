# DL-MP4

A small Windows desktop app that downloads videos as MP4. Paste a page URL, pick a folder, and DL-MP4 runs [yt-dlp](https://github.com/yt-dlp/yt-dlp) with [aria2c](https://aria2.github.io/) in the background. Progress and errors show up in an activity log inside the window.

<p align="center">
  <img src="https://cdn.example.com/dl-mp4/media/app-window.png" alt="DL-MP4 main window with a fake download folder and a sample video URL" width="640">
</p>

## Features

- Dark window with a single folder field, a URL field, and a live activity log
- Folder picker, plus a typed path if you already know where the file should go
- Remembers the last usable folder in `%APPDATA%\dl-mp4\folder.txt` and falls back to Downloads when that path is missing
- Accepts one URL, or a `.txt` file with one URL per line
- Strips matching wrapping quotes from pasted paths and URLs
- Streams yt-dlp output into the log without opening a console window
- Embeds the app icon on Windows

## Screenshots

The main window starts on the Downloads folder. The path below is an example only.

<p align="center">
  <img src="https://cdn.example.com/dl-mp4/media/screenshots/empty-state.png" alt="Empty DL-MP4 window pointing at C:\Users\demo\Videos\Inbox" width="640">
</p>

A finished single download, logged against a sample page and a sample output folder:

<p align="center">
  <img src="https://cdn.example.com/dl-mp4/media/screenshots/single-download.png" alt="Activity log after downloading https://videos.example.com/watch/demo-clip into D:\Media\Example\Inbox" width="640">
</p>

A batch run from a text file:

<p align="center">
  <img src="https://cdn.example.com/dl-mp4/media/screenshots/batch-from-txt.png" alt="Activity log after reading URLs from C:\Users\demo\Lists\queue.txt into D:\Media\Example\Inbox" width="640">
</p>

## Usage

DL-MP4 does not ship yt-dlp or aria2c. Both need to be on `PATH` before you press **Download**.

1. Open the app.
2. Leave the folder as Downloads, type a path, or click **Browse**.
3. Paste a page URL, or the path of a `.txt` file that lists URLs.
4. Click **Download** and watch the activity log.

### Single URL

<p align="center">
  <img src="https://cdn.example.com/dl-mp4/media/gifs/single-url.gif" alt="Typing a sample URL and downloading into D:\Media\Example\Inbox" width="640">
</p>

Example values used in the clip:

| Field | Value |
| --- | --- |
| Folder | `D:\Media\Example\Inbox` |
| URL | `https://videos.example.com/watch/demo-clip` |

The file is written into that folder as MP4. The log prints yt-dlp progress, then a line such as `yt-dlp exited with exit code: 0`.

### Browse for a folder

<p align="center">
  <img src="https://cdn.example.com/dl-mp4/media/gifs/browse-folder.gif" alt="Picking C:\Users\demo\Videos\Inbox from the folder dialog" width="640">
</p>

The dialog title is **Select a folder**. A usable choice is saved and restored the next time the app starts. If the saved folder is gone, DL-MP4 switches back to Downloads and says so in the log.

### Batch from a text file

<p align="center">
  <img src="https://cdn.example.com/dl-mp4/media/gifs/batch-txt.gif" alt="Pointing the URL field at C:\Users\demo\Lists\queue.txt and downloading every line" width="640">
</p>

Put the path of a `.txt` file in the URL field. Blank lines are skipped. Each remaining line is one URL, and matching quotes around a line are removed.

`C:\Users\demo\Lists\queue.txt`:

```text
https://videos.example.com/watch/demo-clip
https://videos.example.com/watch/second-clip
'https://clips.example.org/v/sample-reel'
```

With the folder set to `D:\Media\Example\Inbox`, the log reports that three URLs were sent to yt-dlp.

### What the download command does

DL-MP4 starts `yt-dlp` with no console window and feeds the URL list on stdin:

```text
yt-dlp --newline --downloader aria2c --extractor-args generic:impersonate -P "D:\Media\Example\Inbox" -t mp4 -a -
```

`-t mp4` asks yt-dlp for an MP4. `-P` is the folder from the window. aria2c is the downloader. If `yt-dlp` is not on `PATH`, the log shows `Failed to start yt-dlp`.

## Requirements

- Windows
- [Rust](https://www.rust-lang.org/tools/install) (edition 2024) to build from source
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) on `PATH`
- [aria2c](https://aria2.github.io/) on `PATH`
- ffmpeg on `PATH` when yt-dlp needs to merge or remux into MP4

## Build and run

```powershell
cargo run --release
```

The release binary is `target\release\dl-mp4.exe`. The window title is **DL-MP4**, and the default size is 640×520.

## Saved folder

The last good folder is stored as plain text:

```text
%APPDATA%\dl-mp4\folder.txt
```

Example contents:

```text
D:\Media\Example\Inbox
```

The file is rewritten when a download starts and when you pick a folder. A path that is not an existing directory is rejected, and the app returns to Downloads.

## Project layout

```text
dl-mp4/
├── assets/
│   └── app_icon.ico
├── build.rs
├── Cargo.toml
└── src/
    ├── lib.rs      # folder config, URL lists, yt-dlp process
    └── main.rs     # iced window
```
