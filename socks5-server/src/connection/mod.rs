use self::{associate::Associate, bind::Bind, connect::Connect};
use crate::Auth;
use socks5_proto::{
    Address, Command, HandshakeMethod, HandshakeRequest, HandshakeResponse, Reply, Request,
    Response,
};
use std::{
    io::{Error, ErrorKind, Result},
    sync::Arc,
};
use tokio::{io::AsyncWriteExt, net::TcpStream};

pub mod associate;
pub mod bind;
pub mod connect;

pub struct IncomingConnection {
    stream: TcpStream,
    auth: Arc<dyn Auth + Send + Sync + 'static>,
}

impl IncomingConnection {
    pub(crate) fn new(stream: TcpStream, auth: Arc<dyn Auth + Send + Sync + 'static>) -> Self {
        IncomingConnection { stream, auth }
    }

    pub async fn handshake(mut self) -> Result<Connection> {
        if let Err(err) = self.auth().await {
            let _ = self.stream.shutdown().await;
            return Err(err);
        }

        let req = match Request::read_from(&mut self.stream).await {
            Ok(req) => req,
            Err(err) => {
                let resp = Response::new(Reply::GeneralFailure, Address::unspecified());
                resp.write_to(&mut self.stream).await?;
                let _ = self.stream.shutdown().await;
                return Err(err);
            }
        };

        match req.command {
            Command::Associate => Ok(Connection::Associate(
                Associate::<associate::NeedReply>::new(self.stream),
                req.address,
            )),
            Command::Bind => Ok(Connection::Bind(
                Bind::<bind::NeedFirstReply>::new(self.stream),
                req.address,
            )),
            Command::Connect => Ok(Connection::Connect(
                Connect::<connect::NeedReply>::new(self.stream),
                req.address,
            )),
        }
    }

    async fn auth(&mut self) -> Result<()> {
        let hs_req = HandshakeRequest::read_from(&mut self.stream).await?;
        let chosen_method = self.auth.as_handshake_method();

        if hs_req.methods.contains(&chosen_method) {
            let hs_resp = HandshakeResponse::new(chosen_method);
            hs_resp.write_to(&mut self.stream).await?;
            self.auth.execute(&mut self.stream).await
        } else {
            let hs_resp = HandshakeResponse::new(HandshakeMethod::Unacceptable);
            hs_resp.write_to(&mut self.stream).await?;

            Err(Error::new(
                ErrorKind::Unsupported,
                "No available handshake method provided by client",
            ))
        }
    }
}

pub enum Connection {
    Associate(Associate<associate::NeedReply>, Address),
    Bind(Bind<bind::NeedFirstReply>, Address),
    Connect(Connect<connect::NeedReply>, Address),
}
