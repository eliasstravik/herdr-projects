//! The WebSocket to ElevenLabs: TLS in Rust (rustls, with the web's root certificates built in), one connection per
//! call. Reads wait a few milliseconds at most, so one loop can read, send your audio and play the agent's in turn.
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;
use tungstenite::{Message, WebSocket};

type Tls = rustls::StreamOwned<rustls::ClientConnection, TcpStream>;
/// What the call's loop needs of a connection: send a text, read one if there is one (None: nothing yet), and Err when
/// it closed. A test plays the other side with a fake.
pub trait Wire: Send {
    fn send(&mut self, text: String) -> Result<(), String>;
    fn recv(&mut self) -> Result<Option<String>, String>;
    fn close(&mut self);
}
pub struct Ws(WebSocket<Tls>);
pub fn connect(url: &str) -> Result<Ws, String> {
    let rest = url.strip_prefix("wss://").ok_or("the call's address is not a wss:// one")?;
    let host = rest.split(['/', '?']).next().unwrap_or("").to_string();
    let (name, port) = match host.rsplit_once(':') { Some((h, p)) => (h.to_string(), p.parse::<u16>().unwrap_or(443)), None => (host.clone(), 443) };
    let tcp = TcpStream::connect((name.as_str(), port)).map_err(|e| format!("ElevenLabs could not be reached: {e}"))?;
    let _ = tcp.set_nodelay(true);
    tcp.set_read_timeout(Some(Duration::from_secs(15))).ok();
    let roots = rustls::RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() };
    let config = rustls::ClientConfig::builder().with_root_certificates(roots).with_no_client_auth();
    let server = rustls::pki_types::ServerName::try_from(name.clone()).map_err(|e| e.to_string())?;
    let conn = rustls::ClientConnection::new(Arc::new(config), server).map_err(|e| e.to_string())?;
    let tls = rustls::StreamOwned::new(conn, tcp);
    let (ws, _) = tungstenite::client::client(url, tls).map_err(|e| format!("ElevenLabs refused the call's connection: {e}"))?;
    ws.get_ref().sock.set_read_timeout(Some(Duration::from_millis(8))).ok();
    Ok(Ws(ws))
}
impl Wire for Ws {
    fn send(&mut self, text: String) -> Result<(), String> { self.0.send(Message::text(text)).map_err(|e| e.to_string()) }
    fn recv(&mut self) -> Result<Option<String>, String> {
        match self.0.read() {
            Ok(Message::Text(t)) => Ok(Some(t.to_string())),
            Ok(Message::Close(_)) => Err("the call was closed".into()),
            Ok(_) => Ok(None),
            Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => { let _ = self.0.flush(); Ok(None) }
            Err(e) => Err(e.to_string()),
        }
    }
    fn close(&mut self) { let _ = self.0.close(None); let _ = self.0.flush(); }
}
