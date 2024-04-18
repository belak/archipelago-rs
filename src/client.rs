use std::task::Poll;
use std::{collections::VecDeque, pin::Pin, result::Result};

use anyhow::Context;
use futures::stream::{SplitSink, SplitStream};
use futures::{Sink, SinkExt, Stream, StreamExt};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tungstenite::Message;

use crate::protocol;

const SUPPORTED_VERSION: protocol::NetworkVersion = protocol::NetworkVersion {
    major: 0,
    minor: 4,
    build: 5,
};

pub struct AnonymousClient {
    ws_reader: MessageStream<protocol::AnonymousServerMessage>,
    ws_writer: MessageSink<protocol::ClientMessage>,
    room_info: protocol::RoomInfo,
}

type WsSink = SplitSink<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>, Message>;
type WsStream = SplitStream<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>>;

impl AnonymousClient {
    pub async fn new(url: impl AsRef<str>) -> anyhow::Result<Self> {
        let url = url.as_ref();
        let (host, port) = url
            .rsplit_once(':')
            .map_or_else(|| (url, None), |(url, port)| (url, Some(port)));
        let port = port.unwrap_or("38281");

        // TODO: TLS

        let (ws, _) = connect_async(format!("ws://{}:{}", host, port))
            .await
            .context("failed to connect to websocket")?;

        let (ws_writer, ws_reader) = ws.split();

        let mut ws_reader = MessageStream::new(ws_reader, VecDeque::new());
        let ws_writer = MessageSink::new(ws_writer);

        let room_info = match ws_reader.next().await {
            Some(Ok(protocol::AnonymousServerMessage::RoomInfo(room_info))) => Ok(room_info),
            Some(Ok(_)) => Err(anyhow::anyhow!("expected RoomInfo message")),
            Some(Err(e)) => Err(e.into()),
            None => Err(anyhow::anyhow!("stream unexpectedly ended")),
        }?;

        let ret = Self {
            ws_reader,
            ws_writer,
            room_info,
        };

        Ok(ret)
    }

    pub async fn get_data_package(&mut self) -> anyhow::Result<protocol::DataPackage> {
        self.ws_writer
            .send(protocol::ClientMessage::GetDataPackage(
                protocol::GetDataPackage {
                    games: self.room_info.games.clone(),
                },
            ))
            .await?;

        match self
            .ws_reader
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("stream unexpectedly ended"))??
        {
            protocol::AnonymousServerMessage::DataPackage(data_package) => Ok(data_package),
            _ => Err(anyhow::anyhow!("expected DataPackage message")),
        }
    }

    pub async fn connect(
        mut self,
        password: Option<String>,
        game: impl Into<String>,
        name: impl Into<String>,
        tags: Vec<impl Into<String>>,
        items_handling: protocol::ItemsHandlingFlags,
    ) -> anyhow::Result<Client> {
        let tags = tags.into_iter().map(|tag| tag.into()).collect();

        self.ws_writer
            .send(protocol::ClientMessage::Connect(protocol::Connect {
                password,
                game: game.into(),
                name: name.into(),
                uuid: uuid::Uuid::new_v4().to_string(),
                version: SUPPORTED_VERSION,
                items_handling,
                tags,
                slot_data: true,
            }))
            .await?;

        self.ws_writer.flush().await?;

        let connected = match self
            .ws_reader
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("stream unexpectedly ended"))??
        {
            protocol::AnonymousServerMessage::Connected(connected) => Ok(connected),
            protocol::AnonymousServerMessage::InvalidPacket(invalid) => Err(anyhow::anyhow!(
                "expected Connected message, got InvalidPacket: {:?}",
                invalid
            )),
            protocol::AnonymousServerMessage::ConnectionRefused(refused) => Err(anyhow::anyhow!(
                "expected Connected message, got ConnectionRefused: {:?}",
                refused
            )),
            msg => Err(anyhow::anyhow!("expected Connected message, got {:?}", msg)),
        }?;

        let (ws_reader, message_buffer) = self.ws_reader.into_inner();
        let ws_writer = self.ws_writer.into_inner();
        let room_info = self.room_info;

        Ok(Client {
            ws_reader: MessageStream::new(ws_reader, message_buffer),
            ws_writer: MessageSink::new(ws_writer),
            room_info,
            connected,
        })
    }
}

pub struct Client {
    ws_reader: MessageStream<protocol::ServerMessage>,
    ws_writer: MessageSink<protocol::ClientMessage>,

    room_info: protocol::RoomInfo,
    connected: protocol::Connected,
}

impl Client {
    pub fn get_room_info(&self) -> &protocol::RoomInfo {
        &self.room_info
    }

    pub fn get_connected(&self) -> &protocol::Connected {
        &self.connected
    }
}

impl Stream for Client {
    type Item = Result<protocol::ServerMessage, MessageStreamError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.ws_reader.poll_next_unpin(cx)
    }
}

struct MessageSink<T>
where
    T: serde::ser::Serialize + Unpin,
{
    inner: WsSink,
    phantom: std::marker::PhantomData<T>,
}

impl<T> MessageSink<T>
where
    T: serde::ser::Serialize + Unpin,
{
    fn new(inner: WsSink) -> Self {
        Self {
            inner,
            phantom: std::marker::PhantomData,
        }
    }

    fn into_inner(self) -> WsSink {
        self.inner
    }
}

impl<T> Sink<T> for MessageSink<T>
where
    T: serde::ser::Serialize + Unpin,
{
    type Error = anyhow::Error;

    fn poll_ready(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready_unpin(cx).map_err(Into::into)
    }

    fn start_send(mut self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        let message = Message::text(serde_json::to_string(&[item])?);
        println!("Sending message: {:?}", message);
        self.inner.start_send_unpin(message).map_err(Into::into)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_flush_unpin(cx).map_err(Into::into)
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_close_unpin(cx).map_err(Into::into)
    }
}

struct MessageStream<T>
where
    T: serde::de::DeserializeOwned + Unpin,
{
    inner: WsStream,

    // TODO: this should be a VecDeque of the actual message type, but we don't
    // trust the underlying deserialization yet, as it hasn't been tested on all
    // message types.
    message_buffer: VecDeque<serde_json::Value>,

    phantom: std::marker::PhantomData<T>,
}

impl<T> MessageStream<T>
where
    T: serde::de::DeserializeOwned + Unpin,
{
    fn new(inner: WsStream, message_buffer: VecDeque<serde_json::Value>) -> Self {
        Self {
            inner,
            message_buffer,
            phantom: std::marker::PhantomData,
        }
    }

    fn into_inner(self) -> (WsStream, VecDeque<serde_json::Value>) {
        (self.inner, self.message_buffer)
    }
}

// TODO: shouldn't be pub
#[derive(Debug, thiserror::Error)]
pub enum MessageStreamError {
    #[error("failed to parse message from server: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("websocket error: {0}")]
    WebsocketError(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("got unexpected message type from server: {0}")]
    UnexpectedMessageType(&'static str),
}

impl<T> Stream for MessageStream<T>
where
    T: serde::de::DeserializeOwned + Unpin,
{
    type Item = Result<T, MessageStreamError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // If there are any leftover messages from the last poll, return them
        // first.
        if let Some(message) = self.message_buffer.pop_front() {
            return Poll::Ready(Some(serde_json::from_value(message).map_err(|e| e.into())));
        }

        match self.inner.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(message))) => {
                match message {
                    Message::Text(text) => {
                        // The server can send multiple messages in a single
                        // websocket text response, so we store them to be used
                        // when poll_next is called again.
                        let mut messages: VecDeque<serde_json::Value> =
                            serde_json::from_str(&text)?;

                        let message = match messages.pop_front() {
                            Some(message) => message,
                            None => return Poll::Pending,
                        };

                        self.message_buffer.append(&mut messages);

                        let result = serde_json::from_value(message).map_err(|e| e.into());

                        Poll::Ready(Some(result))
                    }

                    // Ping is handled by the tungstenite library, so we can
                    // effectively ignore them. We don't use pongs, so there's
                    // no point in handling them, but it's not worth erroring.
                    Message::Ping(_) | Message::Pong(_) => Poll::Pending,

                    // If we get a "Close" message, mark this stream as done.
                    //
                    // TODO: maybe this should try an extract the reason.
                    Message::Close(_) => Poll::Ready(None),

                    msg => Poll::Ready(Some(Err(MessageStreamError::UnexpectedMessageType(
                        match msg {
                            Message::Text(_) => "text",
                            Message::Binary(_) => "binary",
                            Message::Ping(_) => "ping",
                            Message::Pong(_) => "pong",
                            Message::Close(_) => "close",
                            Message::Frame(_) => "frame",
                        },
                    )))),
                }
            }
            Poll::Ready(Some(Err(inner))) => Poll::Ready(Some(Err(inner)))?,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
