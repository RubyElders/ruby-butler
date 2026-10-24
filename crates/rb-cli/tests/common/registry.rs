use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

pub struct Registry {
    pub url: String,
    stopped: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Registry {
    pub fn serve(root: &Path) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = stopped.clone();
        let root = root.to_owned();
        let thread = thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => panic!("Registry fixture: {error}"),
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                let mut buffer = [0; 4096];
                while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                    let count = stream.read(&mut buffer).unwrap();
                    if count == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..count]);
                }
                let request = String::from_utf8_lossy(&request);
                let mut words = request.split_whitespace();
                let method = words.next().unwrap_or("");
                let path = words
                    .next()
                    .unwrap_or("/")
                    .split('?')
                    .next()
                    .unwrap()
                    .trim_start_matches('/');
                let body = if path.split('/').any(|part| part == "..") {
                    None
                } else {
                    fs::read(root.join(path)).ok()
                };
                let status = if body.is_some() {
                    "200 OK"
                } else {
                    "404 Not Found"
                };
                let body = body.unwrap_or_default();
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                if method != "HEAD" {
                    let _ = stream.write_all(&body);
                }
            }
        });
        Self {
            url,
            stopped,
            thread: Some(thread),
        }
    }
}

impl Drop for Registry {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}
