use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

const IO_TIMEOUT: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(5);
const HEADER_LIMIT: usize = 16 * 1024;

#[derive(Clone)]
struct Response {
    status: u16,
    body: Vec<u8>,
    length: usize,
}
type Routes = Arc<Mutex<HashMap<String, Response>>>;

pub struct HttpServer {
    pub url: String,
    routes: Routes,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl HttpServer {
    pub fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let routes: Routes = Arc::default();
        let requests: Arc<Mutex<Vec<String>>> = Arc::default();
        let stop = Arc::new(AtomicBool::new(false));
        let (routes_thread, requests_thread, stop_thread) =
            (routes.clone(), requests.clone(), stop.clone());
        let thread = thread::spawn(move || {
            while !stop_thread.load(Ordering::Relaxed) {
                let Ok((mut stream, _)) = listener.accept() else {
                    thread::sleep(POLL_INTERVAL);
                    continue;
                };
                stream.set_nonblocking(false).unwrap();
                stream.set_read_timeout(Some(IO_TIMEOUT)).unwrap();
                stream.set_write_timeout(Some(IO_TIMEOUT)).unwrap();
                let mut request = Vec::new();
                let mut byte = [0];
                while !request.ends_with(b"\r\n\r\n") && request.len() < HEADER_LIMIT {
                    if stream.read(&mut byte).ok() != Some(1) {
                        break;
                    }
                    request.push(byte[0]);
                }
                let request = String::from_utf8_lossy(&request);
                let path = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
                requests_thread.lock().unwrap().push(request.into_owned());
                let response =
                    routes_thread
                        .lock()
                        .unwrap()
                        .get(&path)
                        .cloned()
                        .unwrap_or(Response {
                            status: 404,
                            body: vec![],
                            length: 0,
                        });
                let _ = write!(
                    stream,
                    "HTTP/1.1 {} OK\r\nContent-Length: {}\r\nETag: \"fixture\"\r\nConnection: close\r\n\r\n",
                    response.status, response.length
                );
                let _ = stream.write_all(&response.body);
            }
        });
        Self {
            url,
            routes,
            requests,
            stop,
            thread: Some(thread),
        }
    }
    pub fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }

    pub fn route(&self, path: &str, status: u16, body: impl Into<Vec<u8>>) {
        let body = body.into();
        let length = body.len();
        self.respond(
            path,
            Response {
                status,
                body,
                length,
            },
        );
    }
    pub fn truncated_route(&self, path: &str, length: usize, body: impl Into<Vec<u8>>) {
        self.respond(
            path,
            Response {
                status: 200,
                body: body.into(),
                length,
            },
        );
    }

    fn respond(&self, path: &str, response: Response) {
        self.routes.lock().unwrap().insert(path.into(), response);
    }
}
impl Drop for HttpServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}

impl Default for HttpServer {
    fn default() -> Self {
        Self::new()
    }
}
