extern crate log;
use mio::{Events, Poll, Ready, PollOpt, Token};
use mio::tcp::{TcpListener, TcpStream};
use std::net::{SocketAddr};
use std::collections::{HashMap};
use std::{thread, time};
use std::io::{self, Read, Write};
use super::peer::{PeerContext, PeerDirection};
use super::MSG_BUF_SIZE;
use super::message::{Message, ServerSignal, ConnectResult, ConnectHandle, TaskRequest};
use super::peer::{self, ReadResult, WriteResult};
use mio_extras::channel::{self, Receiver};
use std::sync::mpsc;
use crossbeam::channel as cbchannel;
use mio::{self, net};

use log::{info, warn};

// refer to https://sergey-melnychuk.github.io/2019/08/01/rust-mio-tcp-server/ 
// for context
const LISTENER: Token = Token(0);
const CONTROL: Token = Token(1);
const NETWORK_TOKEN: usize = 0;
const LOCAL_TOKEN: usize = 1;

const EVENT_CAP: usize = 1024;

pub struct Context {
    poll: mio::Poll,
    peers: HashMap<Token, PeerContext>,
    token_counter: usize,
    task_sender: cbchannel::Sender<TaskRequest>,
    response_receiver: HashMap<Token, channel::Receiver<Vec<u8>>>,
    api_receiver: channel::Receiver<ServerSignal>,
    local_addr: SocketAddr,
    is_scale_node: bool,
}

pub struct Handle {
    pub control_tx: channel::Sender<ServerSignal>,
}

impl Handle{
    pub fn connect(&mut self, addr: SocketAddr) -> std::io::Result<mpsc::Receiver<ConnectResult>> {
        let (sender, receiver) = mpsc::channel();
        let connect_handle = ConnectHandle {
            result_sender: sender,
            dest_addr: addr ,
        };
        self.control_tx.send(ServerSignal::ServerConnect(connect_handle.clone()));       
        Ok(receiver)
    }

    pub fn broadcast(
        &mut self, 
        msg: Message
    ) { // -> std::io::Result<mpsc::Receiver<ConnectResult>> {
        self.control_tx.send(
            ServerSignal::ServerBroadcast(msg));       
    }
}

impl Context {
    pub fn new(
        task_sender: cbchannel::Sender<TaskRequest>, 
        addr: SocketAddr, 
        is_scale_node: bool,
    ) -> (Context, Handle) {
        let (control_tx, control_rx) = channel::channel();
        let handle = Handle { 
            control_tx: control_tx,
        };
        let context = Context{
            poll: Poll::new().unwrap(),
            peers: HashMap::new(),
            token_counter: 2, // 0, 1 token are reserved
            task_sender: task_sender,
            response_receiver: HashMap::new(),
            api_receiver: control_rx,
            local_addr: addr,
            is_scale_node: is_scale_node,
        };
        (context, handle)
    }
    
    // start a server, spawn a process
    pub fn start(mut self) {
        let _handler = thread::spawn(move || {
            self.listen(); 
        });
    }

    // register tcp in the event loop
    // network read token i
    // local event token i + 1
    // token starts at 2
    pub fn register_peer(&mut self, socket: TcpStream, direction: PeerDirection) -> io::Result<Token> {
        let peer_addr = socket.peer_addr().unwrap();
        let network_token = Token(self.token_counter);
        self.token_counter += 1;
        
        self.poll.register(
            &socket, 
            network_token.clone(),
            Ready::readable(),
            PollOpt::edge()
        ).unwrap();

        // create a peer context
        let (peer_context, handle) = PeerContext::new(socket, direction).unwrap();
        let local_token = Token(self.token_counter);
        self.token_counter += 1;
        self.poll.register(
            &peer_context.writer.queue,
            local_token,
            Ready::readable(),
            PollOpt::edge() | mio::PollOpt::oneshot(),
        ).unwrap();
        self.peers.insert(network_token, peer_context);
        Ok(network_token)
    }

    // create tcp stream for each peer
    pub fn connect(&mut self, connect_handle: ConnectHandle) -> io::Result<()> {
        let addr: SocketAddr = connect_handle.dest_addr;
        let timeout = time::Duration::from_millis(3000);
        let tcp_stream = match std::net::TcpStream::connect_timeout(&addr, timeout) {
            Ok(s) => s,
            Err(e) => {
                connect_handle.result_sender.send(ConnectResult::Fail);
                return Ok(());
            }
        };
        let stream = TcpStream::from_stream(tcp_stream)?;
        let network_token = self.register_peer(stream, PeerDirection::Outgoing).unwrap();
        connect_handle.result_sender.send(ConnectResult::Success);
        Ok(())
    }

    pub fn process_control(&mut self, msg: ServerSignal) -> std::io::Result<()> {
        match msg {
            ServerSignal::ServerConnect(connect_handle) => {
                self.connect(connect_handle);
            },
            ServerSignal::ServerBroadcast(network_message) => {
                for (token, peer) in self.peers.iter() {
                    match peer.direction {
                        PeerDirection::Incoming => (),
                        PeerDirection::Outgoing => {
                            peer.peer_handle.write(network_message.clone()); 
                        },
                    }
                }
            },
            ServerSignal::ServerUnicast((socket, network_message)) => {
                for (token, peer) in self.peers.iter() { 
                    if peer.addr == socket {
                        match peer.direction {
                            PeerDirection::Incoming => (),
                            PeerDirection::Outgoing => {
                                peer.peer_handle.write(network_message.clone()); 
                            },
                        }
                    }
                }
            },
            ServerSignal::ServerStart => {
            },
            ServerSignal::ServerStop => {
            },
            ServerSignal::ServerDisconnect => {
            },
        }
        Ok(())
    }

    pub fn process_writable(&mut self, token: mio::Token) -> std::io::Result<()> {
        let peer = self.peers.get_mut(&token).expect("writable cannot get peer"); 
        match peer.writer.write() {
            Ok(WriteResult::Complete) => {
                let writer_token = mio::Token(token.0 + 1);
                let socket_token = token;

                self.poll.reregister(
                    &peer.stream,
                    socket_token,
                    mio::Ready::readable(),
                    mio::PollOpt::edge(),
                )?;
                // we're interested in write queue again.
                self.poll.reregister(
                    &peer.writer.queue,
                    writer_token,
                    mio::Ready::readable(),
                    mio::PollOpt::edge() | mio::PollOpt::oneshot(),
                )?;
            },
            Ok(WriteResult::EOF) => {
                info!("Peer {} dropped connection", peer.addr);
                self.peers.remove(&token);
            },
            Ok(WriteResult::ChanClosed) => {
                warn!("Peer {} outgoing queue closed", peer.addr);
                let socket_token = token;
                self.poll.reregister(
                    &peer.stream,
                    socket_token,
                    mio::Ready::readable(),
                    mio::PollOpt::edge(),
                )?;
                self.poll.deregister(&peer.writer.queue)?;
            },
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    trace!("Peer {} finished writing", peer.addr);
                // socket is not ready anymore, stop reading
                } else {
                    warn!("Error writing peer {}, disconnecting: {}", peer.addr, e);
                    self.peers.remove(&token);
                }
            }
        }
        Ok(())
    }

    pub fn process_readable(&mut self, token: mio::Token) {
        let mut peer = self.peers.get_mut(&token).expect("get peer fail"); 
        loop {
            match peer.reader.read() {
                Ok(ReadResult::EOF) => {
                    info!("Peer {} dropped connection", peer.addr);
                    self.peers.remove(&token);
                    //let index = self.peer_list.iter().position(|&x| x == peer_id).unwrap();
                    //self.peer_list.swap_remove(index);
                    break;
                }
                Ok(ReadResult::Continue) => {
                    trace!("Peer {:?} reading continue", token);
                    continue;
                },
                Ok(ReadResult::Message(m)) => {
                    // send task request to performer
                    let msg = bincode::deserialize(&m).unwrap();
                    let performer_task = TaskRequest{
                        peer: Some(peer.peer_handle.clone()), 
                        msg: msg,
                    };
                    self.task_sender.send(performer_task).expect("send request to performer");
                },
                Err(ref e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        trace!("Peer {:?} finished reading", token);
                        // socket is not ready anymore, stop reading
                        break;
                    } else {
                        warn!("Error reading peer {}, disconnecting: {}", peer.addr, e);
                        self.peers.remove(&token);
                        //let index = self.peer_list.iter().position(|&x| x == peer_id).unwrap();
                        //self.peer_list.swap_remove(index);
                        break;
                    }
                }
            }
        }
    }

    // polling events
    pub fn listen(&mut self) -> std::io::Result<()> {
        let listener = TcpListener::bind(&self.local_addr).unwrap(); 
       
        self.poll.register(
            &listener, 
            LISTENER,
            Ready::readable(),
            PollOpt::edge()
        ).unwrap();

        self.poll.register(
            &self.api_receiver, 
            CONTROL,
            Ready::readable(),
            PollOpt::edge()
        ).unwrap();

        let mut events = Events::with_capacity(EVENT_CAP);

        loop {
            self.poll.poll(&mut events, None).expect("unable to poll events"); 
            for event in &events {
                let token = event.token();
                match token {
                    LISTENER => {
                        loop {
                            match listener.accept() { 
                                Ok((socket, socket_addr)) => {
                                    match self.register_peer(socket, PeerDirection::Incoming) {
                                        Ok(_) => (),
                                        Err(e) => {
                                            error!("Error initializaing incoming peer {}: {}", socket_addr, e);
                                        }
                                    }
                                },
                                Err(e) => {
                                    if e.kind() == std::io::ErrorKind::WouldBlock {
                                        break;
                                    } else {
                                        return Err(e);
                                    }
                                }
                            }
                        }
                    },
                    CONTROL => {
                        // process until queue is empty
                        loop {
                            match self.api_receiver.try_recv() { 
                                Ok(msg) => self.process_control(msg).unwrap(),
                                Err(e) => match e {
                                    mpsc::TryRecvError::Empty => break,
                                    mpsc::TryRecvError::Disconnected => {
                                        warn!("P2P server dropped, disconnecting all peers");
                                        self.poll.deregister(&self.api_receiver)?;
                                        break;
                                    }
                                }
                            }
                        }
                    },
                    mio::Token(token_id) => {
                        let token_type: usize = token_id % 2;
                        match token_type {
                            NETWORK_TOKEN => {
                                let readiness = event.readiness();
                                if readiness.is_readable() {
                                    //if !self.peers.contains(token) {
                                        //continue;
                                    //}
                                    self.process_readable(token);
                                }
                                if readiness.is_writable() {
                                    //if !self.peers.contains(token) {
                                        //continue;
                                    //}
                                    self.process_writable(token);
                                }
                            },
                            LOCAL_TOKEN => {
                                let peer_token = Token(token_id - 1);
                                let peer = self.peers.get(&peer_token).expect("cannot get peer with local token"); 
                                self.poll.reregister(
                                    &peer.stream,
                                    peer_token,
                                    mio::Ready::readable() | mio::Ready::writable(),
                                    mio::PollOpt::edge(),
                                ).unwrap();
                            },
                            _ => unreachable!(),
                        }
                    }
                }
            }
        }
    }
}


