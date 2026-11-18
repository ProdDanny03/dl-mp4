#![windows_subsystem = "windows"]

use chrono::Local;
use dl_mp4::{
    downloads_dir, folder_config_file, folder_is_usable, run_ytdlp, save_folder_path,
    strip_wrapping_quotes, urls_from_input,
};
use iced::futures::{SinkExt, StreamExt};
use iced::{
    Alignment, Background, Border, Color, Element, Font, Length, Task, Theme, font,
    widget::{Column, button, column, container, row, scrollable, text, text_input},
};
use rfd::FileDialog;
use std::collections::VecDeque;
use std::{fs, io, thread};

#[derive(Debug, Clone)]
enum Message {
    PathChanged(String),
    UrlChanged(String),
    Browse,
    Download,
    CommandOutput(String),
}

#[derive(Default)]
struct AppState {
    path: String,
    url: String,
    logs: VecDeque<String>,
}

impl AppState {
    fn log(&mut self, message: impl Into<String>) {
        let timestamp = Local::now().format("%H:%M:%S").to_string();
        self.logs
            .push_back(format!("[{}] {}", timestamp, message.into()));
        if self.logs.len() > 200 {
            self.logs.pop_front();
        }
    }
}

fn boot() -> AppState {
    let downloads = downloads_dir();
    let mut state = AppState {
        path: downloads.clone(),
        ..AppState::default()
    };
    let Some(config) = folder_config_file() else {
        return state;
    };
    match fs::read_to_string(config) {
        Ok(saved) if folder_is_usable(&saved) => {
            state.path = strip_wrapping_quotes(saved.trim()).to_string();
        }
        Ok(saved) if !saved.trim().is_empty() => {
            state.log("Saved folder could not be used. Reverted to Downloads.");
            let _ = save_folder_path(&downloads);
        }
        Err(err) if err.kind() != io::ErrorKind::NotFound => {
            state.log(format!(
                "Saved folder could not be read ({err}). Using Downloads."
            ));
        }
        _ => {}
    }
    state
}

fn use_folder(state: &mut AppState, path: String) -> bool {
    if folder_is_usable(&path) {
        state.path = strip_wrapping_quotes(path.trim()).to_string();
        if let Err(err) = save_folder_path(&state.path) {
            state.path = downloads_dir();
            state.log(format!(
                "Error: could not save folder path ({err}). Reverted to Downloads."
            ));
            return false;
        }
        true
    } else {
        state.path = downloads_dir();
        let _ = save_folder_path(&state.path);
        state.log("Error: folder path cannot be used. Reverted to Downloads.");
        false
    }
}

fn update(state: &mut AppState, message: Message) -> Task<Message> {
    match message {
        Message::PathChanged(new_path) => {
            state.path = new_path;
            Task::none()
        }
        Message::UrlChanged(new_url) => {
            state.url = new_url;
            Task::none()
        }
        Message::Browse => {
            state.log("Opening folder picker dialog...");
            if let Some(folder) = FileDialog::new().set_title("Select a folder").pick_folder() {
                let path_str = folder.display().to_string();
                if use_folder(state, path_str.clone()) {
                    state.log(format!("Selected folder: {}", path_str));
                }
            } else {
                state.log("Folder selection was cancelled.");
            }
            Task::none()
        }
        Message::Download => {
            let url = strip_wrapping_quotes(state.url.trim()).to_string();
            let path = strip_wrapping_quotes(state.path.trim()).to_string();
            if url.is_empty() {
                state.log("Error: No URL specified. Please enter a URL.");
                return Task::none();
            }
            if !use_folder(state, path) {
                return Task::none();
            }
            let path = state.path.clone();
            let urls = match urls_from_input(&url) {
                Ok(urls) => urls,
                Err(err) => {
                    state.log(err);
                    return Task::none();
                }
            };
            if urls.len() == 1 && urls[0] == url {
                state.log(format!(
                    "Requesting to download '{}' via yt-dlp to location: {}",
                    url, path
                ));
            } else {
                state.log(format!(
                    "Requesting to download {} URLs from '{}' via yt-dlp to location: {}",
                    urls.len(),
                    url,
                    path
                ));
            }
            download_task(path, urls)
        }
        Message::CommandOutput(line) => {
            state.log(line);
            Task::none()
        }
    }
}

fn download_task(path: String, urls: Vec<String>) -> Task<Message> {
    Task::run(
        iced::stream::channel(64, async move |mut sender| {
            let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
            thread::spawn(move || {
                let emit = move |line| {
                    let _ = tx.unbounded_send(line);
                };
                run_ytdlp(&path, &urls, emit);
            });

            while let Some(line) = rx.next().await {
                if sender.send(line).await.is_err() {
                    break;
                }
            }
        }),
        Message::CommandOutput,
    )
}

fn view(state: &AppState) -> Column<'_, Message> {
    let path_input = text_input("Enter folder path or click Browse...", &state.path)
        .on_input(Message::PathChanged)
        .width(Length::Fill)
        .padding(10);

    let url_input = text_input("Enter URL or .txt file path...", &state.url)
        .on_input(Message::UrlChanged)
        .width(Length::Fill)
        .padding(10);

    let browse_btn = button(text("Browse").center().width(Length::Fill))
        .on_press(Message::Browse)
        .padding(10)
        .width(ACTION_BUTTON_WIDTH);

    let download_btn = button(text("Download").center().width(Length::Fill))
        .on_press(Message::Download)
        .padding(10)
        .width(ACTION_BUTTON_WIDTH);

    let path_row = row![path_input, browse_btn]
        .spacing(10)
        .align_y(Alignment::Center);

    let url_row = row![url_input, download_btn]
        .spacing(10)
        .align_y(Alignment::Center);

    let log_entries: Vec<Element<Message>> = state
        .logs
        .iter()
        .map(|entry| text(entry).size(12).into())
        .collect();

    let log_column = column(log_entries).spacing(4).padding(5);

    let log_scroll = scrollable(log_column)
        .height(Length::Fill)
        .width(Length::Fill)
        .anchor_bottom();

    let log_container = container(log_scroll)
        .padding(10)
        .height(Length::Fill)
        .width(Length::Fill)
        .style(log_container_style);

    column![
        text("DL-MP4").size(40).font(Font {
            weight: font::Weight::Bold,
            style: font::Style::Italic,
            ..Font::DEFAULT
        }),
        text("Select or enter a folder path and URL, then click Download to download mp4").size(14),
        path_row,
        url_row,
        text("Activity log:").size(16),
        log_container,
    ]
    .spacing(12)
    .padding(20)
}

const ACTION_BUTTON_WIDTH: f32 = 120.0;

fn log_container_style(theme: &Theme) -> container::Style {
    let background = theme.palette().background;
    container::Style {
        background: Some(Background::Color(Color {
            r: background.r * 0.72,
            g: background.g * 0.72,
            b: background.b * 0.72,
            a: 1.0,
        })),
        border: Border {
            color: Color {
                r: (background.r + 0.18).min(1.0),
                g: (background.g + 0.18).min(1.0),
                b: (background.b + 0.18).min(1.0),
                a: 1.0,
            },
            width: 1.0,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

fn hot_pink_theme() -> Theme {
    let mut palette = Theme::Dark.palette();
    palette.primary = Color::from_rgb8(255, 20, 147);
    Theme::custom("Dark Hot Pink", palette)
}

fn main() -> iced::Result {
    let icon = iced::window::icon::from_file_data(include_bytes!("../assets/app_icon.ico"), None)
        .expect("failed to load app_icon.ico");

    iced::application(boot, update, view)
        .title("DL-MP4")
        .theme(hot_pink_theme())
        .window(iced::window::Settings {
            size: iced::Size::new(640.0, 520.0),
            icon: Some(icon),
            ..iced::window::Settings::default()
        })
        .run()
}
