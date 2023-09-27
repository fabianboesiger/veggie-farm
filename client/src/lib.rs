use itertools::Itertools;
use seed::{prelude::*, *};
use shared::{Event, EventData, Farm, Field, Req, Res, SyncData, Veggie, Silo};
use std::{collections::HashMap, path::PathBuf, rc::Rc, iter::once};
use strum::Display;

#[cfg(not(debug_assertions))]
const WS_URL: &str = "ws://boesiger.internet-box.ch/game/ws";
#[cfg(debug_assertions)]
const WS_URL: &str = "ws://127.0.0.1:3000/game/ws";

// ------ ------
//     Model
// ------ ------

pub struct Model {
    web_socket: WebSocket,
    web_socket_reconnector: Option<StreamHandle>,
    state: Option<SyncData>,
}

// ------ ------
//     Init
// ------ ------

fn init(_url: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders.subscribe(|subs::UrlRequested(_url, url_request)| url_request.handled());

    Model {
        web_socket: create_websocket(orders),
        web_socket_reconnector: None,
        state: None,
    }
}

// ------ ------
//    Update
// ------ ------

#[derive(Debug)]
pub enum Msg {
    WebSocketOpened,
    CloseWebSocket,
    WebSocketClosed(CloseEvent),
    WebSocketFailed,
    ReconnectWebSocket(usize),
    SendGameEvent(Event),
    ReceiveGameEvent(EventData),
    InitGameState(SyncData),
}

fn update(msg: Msg, mut model: &mut Model, orders: &mut impl Orders<Msg>) {
    let web_socket = &model.web_socket;
    let send = |event| {
        let serialized = rmp_serde::to_vec(&Req::Event(event)).unwrap();
        web_socket.send_bytes(&serialized).unwrap();
    };

    match msg {
        Msg::WebSocketOpened => {
            model.web_socket_reconnector = None;
            log!("WebSocket connection is open now");
        }
        Msg::CloseWebSocket => {
            model.web_socket_reconnector = None;
            model
                .web_socket
                .close(None, Some("user clicked close button"))
                .unwrap();
        }
        Msg::WebSocketClosed(close_event) => {
            log!(
                "WebSocket connection was closed, reason:",
                close_event.reason()
            );

            // Chrome doesn't invoke `on_error` when the connection is lost.
            if (!close_event.was_clean() || close_event.code() == 4000)
                && model.web_socket_reconnector.is_none()
            {
                model.web_socket_reconnector = Some(
                    orders.stream_with_handle(streams::backoff(None, Msg::ReconnectWebSocket)),
                );
            }
        }
        Msg::WebSocketFailed => {
            log!("WebSocket failed");
            if model.web_socket_reconnector.is_none() {
                model.web_socket_reconnector = Some(
                    orders.stream_with_handle(streams::backoff(None, Msg::ReconnectWebSocket)),
                );
            }
        }
        Msg::ReconnectWebSocket(retries) => {
            log!("Reconnect attempt:", retries);
            model.web_socket = create_websocket(orders);
        }
        Msg::SendGameEvent(event) => send(event),
        Msg::ReceiveGameEvent(event) => {
            if let Some(SyncData { state, .. }) = &mut model.state {
                if state.update(event).is_none() {
                    web_socket.close(Some(4000), Some("invalid state")).unwrap();
                }
            }
        }
        Msg::InitGameState(sync_data) => {
            model.state = Some(sync_data);
        }
    }
}

fn create_websocket(orders: &impl Orders<Msg>) -> WebSocket {
    let msg_sender = orders.msg_sender();

    WebSocket::builder(WS_URL, orders)
        .on_open(|| Msg::WebSocketOpened)
        .on_message(move |msg| decode_message(msg, msg_sender))
        .on_close(Msg::WebSocketClosed)
        .on_error(|| Msg::WebSocketFailed)
        .build_and_open()
        .unwrap()
}

fn decode_message(message: WebSocketMessage, msg_sender: Rc<dyn Fn(Option<Msg>)>) {
    if message.contains_text() {
        unreachable!()
    } else {
        spawn_local(async move {
            let bytes = message
                .bytes()
                .await
                .expect("WebsocketError on binary data");

            let msg: Res = rmp_serde::from_slice(&bytes).unwrap();
            match msg {
                Res::Event(event) => {
                    msg_sender(Some(Msg::ReceiveGameEvent(event)));
                }
                Res::Sync(sync) => {
                    msg_sender(Some(Msg::InitGameState(sync)));
                }
            }
        });
    }
}

// ------ ------
//     View
// ------ ------

fn view(model: &Model) -> Node<Msg> {
    if let Some(data) = &model.state {

        let player = data.state.players.get(&data.user_id).unwrap();
        div![
            p![format!("user id, {}", data.user_id)],
            div![
                C!["grid"],
                player.farm.render().into_iter().map(|draw| div![attrs!(
                    At::Style => draw.style()
                )])
            ]
        ]
    } else {
        div![C!["loading"], "Loading ..."]
    }
}

// ------ ------
//     Start
// ------ ------

#[wasm_bindgen(start)]
pub fn start() {
    App::start("app", init, update, view);
}

trait Render {
    fn render(&self) -> Vec<Draw>
    where
        Self: Sized;
}

impl Render for Farm {
    fn render(&self) -> Vec<Draw>
    where
        Self: Sized,
    {
        let fields = self.fields
            .iter()
            .enumerate()
            .flat_map(|(i, f)| {
                f.render()
                    .into_iter()
                    .map(move |d| d.mov((i as i32 * 3) % 9, 1 + (i as i32 * 3) / 9, 0))
            });

        let silos = self.silos
            .iter()
            .enumerate()
            .flat_map(|(i, f)| {
                f.render()
                    .into_iter()
                    .map(move |d| d.mov(i as i32 + 3, i as i32 + 5, 0))
            });
        
        silos
            .chain(fields)
            .collect()
    }
}

impl Render for Field {
    fn render(&self) -> Vec<Draw>
    where
        Self: Sized,
    {
        vec![Draw {
            x: 0,
            y: 0,
            z: 0,
            texture: Texture::Field,
        }]
    }
}

impl Render for Silo {
    fn render(&self) -> Vec<Draw>
    where
        Self: Sized,
    {
        let back = once(Draw {
            x: 0,
            y: 0,
            z: 0,
            texture: Texture::SiloBackBottom,
        })
            .chain((1..(self.max_storage - 1)).map(|i| Draw {
                x: 0,
                y: -(i as i32),
                z: 0,
                texture: Texture::SiloBackMiddle
            }))
            .chain(once(Draw {
                x: 0,
                y: -(self.max_storage as i32 - 1),
                z: 0,
                texture: Texture::SiloBackTop
            }));

        let front = once(Draw {
            x: 0,
            y: 0,
            z: 2,
            texture: Texture::SiloFrontBottom,
        })
            .chain((1..(self.max_storage - 1)).map(|i| Draw {
                x: 0,
                y: -(i as i32),
                z: 2,
                texture: Texture::SiloFrontMiddle
            }))
            .chain(once(Draw {
                x: 0,
                y: -(self.max_storage as i32 - 1),
                z: 2,
                texture: Texture::SiloFrontTop
            }));

        let veggies = self.storage
            .iter()
            .enumerate()
            .map(|(i, veggie)| {
                Draw {
                    x: 0,
                    y: -(i as i32),
                    z: 1,
                    texture: Texture::Veggie(veggie.veggie())
                }
            });

        back
            .chain(veggies)
            .chain(front)
            .collect()
    }
}

struct Draw {
    x: i32,
    y: i32,
    z: i32,
    texture: Texture,
}

impl Draw {
    fn style(&self) -> String {
        format!(
            r#"
            grid-column: {} / span {};
            grid-row: {} / span {};
            z-index: {};
            background-image: url("/assets/{}");
            aspect-ratio: {};
        "#,
            self.x + 1,
            self.texture.size().0,
            self.y + 1,
            self.texture.size().1,
            self.z + 1,
            self.texture.path().to_string_lossy(),
            self.texture.size().0 as f32 / self.texture.size().1 as f32
        )
    }

    fn mov(mut self, dx: i32, dy: i32, dz: i32) -> Self {
        self.x += dx;
        self.y += dy;
        self.z += dz;
        self
    }
}

#[derive(Display)]
#[strum(serialize_all = "title_case")]
enum Texture {
    Field,
    SiloBackTop,
    SiloBackMiddle,
    SiloBackBottom,
    SiloFrontTop,
    SiloFrontMiddle,
    SiloFrontBottom,
    Veggie(Veggie),
}

impl Texture {
    fn path(&self) -> PathBuf {
        match self {
            Self::Veggie(veggie) => PathBuf::from(veggie.to_string().replace(" ", "-").to_lowercase())
                .with_extension("png"),
            _ => PathBuf::from(self.to_string().replace(" ", "-").to_lowercase())
                .with_extension("png"),
        }
    }

    fn size(&self) -> (u32, u32) {
        match self {
            Self::Field => (3, 3),
            _ => (1, 1),
        }
    }
}
