use crossbeam_channel::{Receiver, Sender, TryRecvError};
use sdl2::{pixels::Color, rect::Rect, render::Texture};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashMap};
use thiserror::Error;
use tracing::{event, instrument, span, Level};

use crate::transport;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Event {
    PasteArray { block: Vec<i32>, i: i32, j: i32 },
    RepaintIt,
    ShowText { text: String },
    SetAbsDimensions { width: i32, height: i32 },
    Exit,
}

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum Error {
    #[error("an error occurring sending message to transport")]
    TransportError(#[from] crossbeam_channel::SendError<transport::Event>),
    #[error("the decoders channel disconnected")]
    DecoderDisconnected,
}

const DISPLAY_PADDING: u32 = 1;

fn rect_eq(first: &Rect, second: &Rect) -> bool {
    first.x() == second.x()
        && first.y() == second.y()
        && first.width() == second.width()
        && first.height() == second.height()
}

#[instrument(skip(rx, transport_tx))]
pub fn handle(rx: Receiver<Event>, transport_tx: Sender<transport::Event>) -> Result<(), Error> {
    let sdl_context = sdl2::init().unwrap();
    let sdl_video = sdl_context.video().unwrap();
    let mut sdl_timer = sdl_context.timer().unwrap();
    let mut sdl_event_pump = sdl_context.event_pump().unwrap();
    let window_width: u32 = 1024 + 2;
    let window_height: u32 = 768 + 2;
    let sdl_window = sdl_video
        .window("", window_width, window_height)
        .opengl()
        .position_centered()
        .resizable()
        .allow_highdpi()
        .build()
        .unwrap();
    let mut sdl_canvas = sdl_window
        .into_canvas()
        .build()
        .expect("failed to build window's canvas");
    sdl_canvas.set_draw_color(Color::RGB(0x16, 0x16, 0x16));
    let sdl_texture_creator = sdl_canvas.texture_creator();

    // screen size is the remote requested size
    let mut screen_rect = Rect::new(0, 0, window_width, window_height);

    // the visible are for displaying screen
    let (display_width, display_height) = sdl_canvas.output_size().unwrap();
    let mut display_rect = Rect::new(0, 0, display_width, display_height);

    // canvas size is the size of canvas on local display
    let mut canvas_rect = calc_canvas_rect(&display_rect, &screen_rect);
    let mut textures_map: HashMap<Rect, Texture> = HashMap::new();
    let mut sdl_texture: Texture = sdl_texture_creator
        .create_texture_streaming(
            sdl2::pixels::PixelFormatEnum::RGB24,
            screen_rect.width(),
            screen_rect.height(),
        )
        .unwrap();
    sdl_canvas
        .copy(&sdl_texture, None, Some(canvas_rect))
        .unwrap();
    sdl_canvas.clear();
    sdl_canvas.present();

    event!(Level::INFO, "graphics setup");

    let gui_receive = span!(Level::DEBUG, "gui_receive");
    let gui_sent = span!(Level::DEBUG, "gui_sent");

    'running: loop {
        const FRAME_MS: u32 = 1000 / 60;
        let ticks = sdl_timer.ticks();
        {
            let _guard = gui_receive.enter();
            match rx.try_recv() {
                Ok(ev) => {
                    use Event::*;
                    match ev {
                        PasteArray { block, i, j } => {
                            event!(Level::DEBUG, "past array");

                            #[allow(unused_mut)]
                            let mut b: Vec<u8> = block
                                .into_iter()
                                .flat_map(|v| (v as u32).to_be_bytes()[1..=3].to_vec())
                                .collect();

                            // handle blocks with less then 16 height
                            let block_height = if j + 16 > (screen_rect.height() as i32) {
                                (screen_rect.height() as i32) - j
                            } else {
                                16
                            };

                            let block_rect = Rect::new(i, j, 16, block_height as u32);

                            sdl_texture.update(block_rect, &b, 3 * 16).unwrap();
                        }
                        RepaintIt => {
                            event!(Level::DEBUG, "Repaint it");
                            sdl_canvas.clear();
                            sdl_canvas
                                .copy(&sdl_texture, None, Some(canvas_rect))
                                .unwrap();
                            sdl_canvas.present();
                        }
                        ShowText { text } => {
                            event!(Level::INFO, "Show text {}", text);
                            sdl_canvas.window_mut().set_title(&text).unwrap();
                        }
                        SetAbsDimensions { width, height } => {
                            event!(Level::INFO, "Set abs dimensions ({},{})", width, height);
                            let new_screen_rect = Rect::new(0, 0, width as u32, height as u32);

                            // check if display resolution changed
                            if rect_eq(&new_screen_rect, &screen_rect) {
                                textures_map.insert(screen_rect, sdl_texture);
                                screen_rect = new_screen_rect;
                                if let Some(mut texture) = textures_map.remove(&screen_rect) {
                                    let b = vec![
                                        0;
                                        (screen_rect.height() * screen_rect.width() * 3)
                                            as usize
                                    ];
                                    texture.update(screen_rect, &b, 3 * 16).unwrap();
                                    sdl_texture = texture;
                                } else {
                                    sdl_texture = sdl_texture_creator
                                        .create_texture_streaming(
                                            sdl2::pixels::PixelFormatEnum::RGB24,
                                            screen_rect.width(),
                                            screen_rect.height(),
                                        )
                                        .unwrap();
                                }
                                let (w, h) = sdl_canvas.output_size().unwrap();
                                display_rect.set_width(w);
                                display_rect.set_height(h);

                                canvas_rect = calc_canvas_rect(&display_rect, &screen_rect);
                                sdl_canvas.clear();
                                sdl_canvas
                                    .copy(&sdl_texture, None, Some(canvas_rect))
                                    .unwrap();
                            }
                        }
                        Exit => break 'running,
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => return Err(Error::DecoderDisconnected),
            }
        }

        if sdl_timer.ticks() - ticks > FRAME_MS {
            continue 'running;
        }

        {
            let _guard = gui_sent.enter();
            for event in sdl_event_pump.poll_iter() {
                use sdl2::event::Event;
                use sdl2::event::WindowEvent;
                match event {
                    Event::Quit { .. } => break 'running,
                    Event::Window {
                        win_event: WindowEvent::Resized(width, height),
                        ..
                    }
                    | Event::Window {
                        win_event: WindowEvent::SizeChanged(width, height),
                        ..
                    } => {
                        let (new_display_width, new_display_height) =
                            sdl_canvas.output_size().unwrap();
                        event!(
                            Level::DEBUG,
                            width,
                            height,
                            new_display_width,
                            new_display_height
                        );
                        display_rect.set_width(new_display_width);
                        display_rect.set_height(new_display_height);

                        canvas_rect = calc_canvas_rect(&display_rect, &screen_rect);

                        sdl_canvas.clear();
                        sdl_canvas
                            .copy(&sdl_texture, None, Some(canvas_rect))
                            .unwrap();
                        sdl_canvas.present();
                    }
                    Event::Window {
                        win_event: WindowEvent::Exposed,
                        ..
                    } => {
                        sdl_canvas
                            .copy(
                                &sdl_texture,
                                None,
                                Some(Rect::new(0, 0, window_width, window_height)),
                            )
                            .unwrap();
                    }
                    Event::KeyDown {
                        keycode: Some(keycode),
                        keymod,
                        repeat,
                        ..
                    } => {
                        event!(Level::DEBUG, ?keycode, ?keymod, "KeyDown");
                        // grab doesn't work for special keys
                        transport_tx.send(transport::Event::KeyPressed {
                            keycode,
                            keymod,
                            repeat,
                        })?;
                    }
                    Event::KeyUp {
                        keycode: Some(keycode),
                        keymod,
                        repeat,
                        ..
                    } => {
                        event!(Level::DEBUG, ?keycode, ?keymod, "KeyUp");
                        transport_tx.send(transport::Event::KeyReleased {
                            keycode,
                            keymod,
                            repeat,
                        })?;
                    }
                    Event::TextInput { text, .. } => {
                        event!(Level::DEBUG, ?text, "TextInput");
                        transport_tx.send(transport::Event::KeyTyped(text))?;
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn calc_canvas_rect(display: &Rect, screen: &Rect) -> Rect {
    let padded_screen_width = screen.width() + DISPLAY_PADDING * 2;
    let padded_screen_height = screen.height() + DISPLAY_PADDING * 2;
    let screen_ratio = padded_screen_width * display.height();
    let display_ratio = display.width() * padded_screen_height;
    event!(Level::TRACE, screen_ratio, display_ratio);
    let (new_padded_canvas_width, new_padded_canvas_height) = match display_ratio.cmp(&screen_ratio)
    {
        Ordering::Greater => {
            event!(Level::DEBUG, "scaling width");
            (
                padded_screen_width * display.height() / padded_screen_height,
                display.height(),
            )
        }
        Ordering::Less => {
            event!(Level::DEBUG, "scaling height");
            (
                display.width(),
                padded_screen_height * display.width() / padded_screen_width,
            )
        }
        Ordering::Equal => {
            event!(Level::DEBUG, "using display dimensions");
            (display.width(), display.height())
        }
    };
    let canvas_width = new_padded_canvas_width - DISPLAY_PADDING * 2;
    let canvas_height = new_padded_canvas_height - DISPLAY_PADDING * 2;
    let canvas_top = ((display.height() - canvas_height) / 2) as i32;
    // NOTE: don't know why adding 2 is necessary to properly center
    let canvas_left = ((display.width() - canvas_width) / 2) as i32 + 2;

    let canvas_rect = Rect::new(canvas_left, canvas_top, canvas_width, canvas_height);
    event!(Level::DEBUG, ?canvas_rect,);
    canvas_rect
}
