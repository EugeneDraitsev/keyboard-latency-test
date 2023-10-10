#![windows_subsystem = "windows"]

use druid::widget::prelude::*;
use druid::widget::{Controller, CrossAxisAlignment, Flex, Label, List, Scroll, SizedBox};
use druid::{
    theme, AppLauncher, Color, Data, FontDescriptor, KeyEvent, Lens, MouseEvent, WidgetExt,
    WindowDesc,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

const CURSOR_BACKGROUND_COLOR: Color = Color::grey8(0x55);
const HEADER_BACKGROUND: Color = Color::grey8(0xCC);
const INTERACTIVE_AREA_DIM: f64 = 160.0;
const INTERACTIVE_AREA_BORDER: Color = Color::grey8(0xCC);
const TEXT_COLOR: Color = Color::grey8(0x11);
const PROPERTIES: &[(&str, f64)] = &[("Duration", 140.0), ("Event", 80.0), ("Key Code", 80.0)];

#[allow(clippy::rc_buffer)]
#[derive(Clone, Data, Lens)]
struct AppState {
    latencies: Arc<Vec<Latencies>>,
    keys_pressed_time: Arc<HashMap<String, u128>>,
}

struct EventLogger<F: Fn(&Event) -> bool> {
    filter: F,
}

impl<W: Widget<AppState>, F: Fn(&Event) -> bool> Controller<AppState, W> for EventLogger<F> {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        if (self.filter)(event) {
            match event {
                Event::KeyDown(key_event) => {
                    match data.keys_pressed_time.get(&key_event.key.to_string()) {
                        Some(_) => {}
                        None => {
                            Arc::make_mut(&mut data.keys_pressed_time).insert(
                                key_event.key.to_string(),
                                SystemTime::now()
                                    .duration_since(SystemTime::UNIX_EPOCH)
                                    .unwrap()
                                    .as_nanos(),
                            );
                        }
                    }
                }
                Event::KeyUp(key_event) => {
                    if let Some(key_pressed_time) =
                        data.keys_pressed_time.get(&key_event.key.to_string())
                    {
                        let key_released_time = SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_nanos();
                        let duration = key_released_time - key_pressed_time;

                        if let Some(to_log) = Latencies::try_from_event(event, duration) {
                            Arc::make_mut(&mut data.latencies).insert(0, to_log);
                        }
                        Arc::make_mut(&mut data.keys_pressed_time)
                            .remove(&key_event.key.to_string());
                    }
                }
                Event::MouseDown(mouse_event) => {
                    match data
                        .keys_pressed_time
                        .get(format!("{:?}", &mouse_event.button).as_str())
                    {
                        Some(_) => {}
                        None => {
                            Arc::make_mut(&mut data.keys_pressed_time).insert(
                                format!("{:?}", &mouse_event.button),
                                SystemTime::now()
                                    .duration_since(SystemTime::UNIX_EPOCH)
                                    .unwrap()
                                    .as_nanos(),
                            );
                        }
                    }
                }
                Event::MouseUp(mouse_event) => {
                    if let Some(mouse_key_pressed_time) = data
                        .keys_pressed_time
                        .get(format!("{:?}", &mouse_event.button).as_str())
                    {
                        let key_released_time = SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_nanos();
                        let duration = key_released_time - mouse_key_pressed_time;

                        if let Some(to_log) = Latencies::try_from_event(event, duration) {
                            Arc::make_mut(&mut data.latencies).insert(0, to_log);
                        }
                        Arc::make_mut(&mut data.keys_pressed_time)
                            .remove(format!("{:?}", &mouse_event.button).as_str());
                    }
                }
                _ => (),
            }
        }
        // Always request focus to receive keyboard events.
        ctx.request_focus();
        // Always pass on the event!
        child.event(ctx, event, data, env)
    }
}

/// The types of events we display
#[derive(Clone, Copy, Data, PartialEq)]
enum EventType {
    KeyDown,
    KeyUp,
    MouseDown,
    MouseUp,
}

#[derive(Clone, Data)]
struct Latencies {
    typ: EventType,
    duration: u128,
    // To see what #[data(ignore)] does look at the docs.rs page on `Data`:
    // https://docs.rs/druid/latest/druid/trait.Data.html
    #[data(ignore)]
    mouse: Option<MouseEvent>,
    #[data(ignore)]
    key: Option<KeyEvent>,
}

impl Latencies {
    fn try_from_event(event: &Event, duration: u128) -> Option<Self> {
        let to_log = match event {
            Event::MouseUp(mouse) => Some((EventType::MouseUp, Some(mouse.clone()), None)),
            Event::MouseDown(mouse) => Some((EventType::MouseDown, Some(mouse.clone()), None)),
            Event::KeyUp(key) => Some((EventType::KeyUp, None, Some(key.clone()))),
            Event::KeyDown(key) => Some((EventType::KeyDown, None, Some(key.clone()))),
            _ => None,
        };

        to_log.map(|(typ, mouse, key)| Latencies {
            typ,
            mouse,
            key,
            duration,
        })
    }

    fn duration(&self) -> String {
        format!("{:?}", Duration::from_nanos(self.duration as u64))
    }

    fn event_name(&self) -> String {
        match self.typ {
            EventType::KeyDown => "KeyDown",
            EventType::KeyUp => "KeyUp",
            EventType::MouseDown => "MouseDown",
            EventType::MouseUp => "MouseUp",
        }
        .into()
    }

    fn code(&self) -> String {
        match self.key.as_ref() {
            Some(key) => key.code.to_string(),
            None => match self.mouse.as_ref() {
                Some(mouse) => format!("{:?}", mouse.button),
                None => "".into(),
            },
        }
    }
}

fn build_root_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_child(interactive_area())
        .with_flex_child(event_list(), 1.0)
}

/// The top part of the application, that accepts keyboard and mouse input.
fn interactive_area() -> impl Widget<AppState> {
    let event_controller = SizedBox::empty()
        .fix_size(INTERACTIVE_AREA_DIM, INTERACTIVE_AREA_DIM)
        .background(CURSOR_BACKGROUND_COLOR)
        .rounded(5.0)
        .border(INTERACTIVE_AREA_BORDER, 1.0)
        .controller(EventLogger {
            filter: |event| {
                matches!(
                    event,
                    Event::KeyDown(_) | Event::KeyUp(_) | Event::MouseDown(_) | Event::MouseUp(_)
                )
            },
        });

    Flex::row().with_child(event_controller).padding(15.0)
}

/// The bottom part of the application, a list of received events.
fn event_list() -> impl Widget<AppState> {
    // Because this would be a HUGE block of repeated code with constants
    // we just use a loop to generate the header.
    let mut header = Flex::row().with_child(
        Label::new(PROPERTIES[0].0)
            .fix_width(PROPERTIES[0].1)
            .background(HEADER_BACKGROUND),
    );

    for (name, size) in PROPERTIES.iter().skip(1) {
        // Keep in mind that later on, in the main function,
        // we set the default spacer values. Without explicitly
        // setting them the default spacer is bigger, and is
        // probably not desirable for your purposes.
        header.add_default_spacer();
        header.add_child(
            Label::new(*name)
                .fix_width(*size)
                .background(HEADER_BACKGROUND),
        );
    }
    Scroll::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(header)
            .with_default_spacer()
            .with_flex_child(
                // `List::new` generates a list entry for every element in the `Vec`.
                // In this case it shows a log entry for every element in `AppState::events`.
                // `make_list_item` generates this new log entry.
                Scroll::new(List::new(make_list_item).lens(AppState::latencies)).vertical(),
                1.0,
            )
            .background(Color::WHITE),
    )
    .horizontal()
}

/// A single event row.
fn make_list_item() -> impl Widget<Latencies> {
    Flex::row()
        .with_child(Label::dynamic(|d: &Latencies, _| d.duration()).fix_width(PROPERTIES[0].1))
        .with_default_spacer()
        .with_child(Label::dynamic(|d: &Latencies, _| d.event_name()).fix_width(PROPERTIES[1].1))
        .with_default_spacer()
        .with_child(Label::dynamic(|d: &Latencies, _| d.code()).fix_width(PROPERTIES[2].1))
}

pub fn main() {
    //describe the main window
    let main_window = WindowDesc::new(build_root_widget())
        .title("Event Viewer")
        .window_size((760.0, 680.0));

    //start the application
    AppLauncher::with_window(main_window)
        .log_to_console()
        .configure_env(|env, _| {
            env.set(theme::UI_FONT, FontDescriptor::default().with_size(12.0));
            env.set(theme::TEXT_COLOR, TEXT_COLOR);
            env.set(theme::WIDGET_PADDING_HORIZONTAL, 2.0);
            env.set(theme::WIDGET_PADDING_VERTICAL, 2.0);
        })
        .launch(AppState {
            latencies: Arc::new(Vec::new()),
            keys_pressed_time: Arc::new(HashMap::new()),
        })
        .expect("Failed to launch application");
}
