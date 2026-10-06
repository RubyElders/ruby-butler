use super::*;

pub(super) struct Server {
    pub(super) source: String,
    pub(super) responses: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    pub(super) requests: Arc<Mutex<Vec<String>>>,
    pub(super) headers: Arc<Mutex<BTreeMap<String, String>>>,
    pub(super) statuses: Arc<Mutex<BTreeMap<String, String>>>,
    pub(super) revalidate: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Server {
    pub(super) fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let source = format!("http://{}/", listener.local_addr().unwrap());
        let responses = Arc::new(Mutex::new(BTreeMap::<String, Vec<u8>>::new()));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let headers = Arc::new(Mutex::new(BTreeMap::<String, String>::new()));
        let statuses = Arc::new(Mutex::new(BTreeMap::<String, String>::new()));
        let revalidate = Arc::new(AtomicBool::new(true));
        let (response_headers, response_statuses, conditional) =
            (headers.clone(), statuses.clone(), revalidate.clone());
        let running = Arc::new(AtomicBool::new(true));
        let (data, log, active) = (responses.clone(), requests.clone(), running.clone());
        let thread = thread::spawn(move || {
            while active.load(Ordering::Relaxed) {
                let Ok((mut stream, _)) = listener.accept() else {
                    thread::sleep(Duration::from_millis(2));
                    continue;
                };
                let request = read_request(&mut stream).unwrap();
                let request = String::from_utf8_lossy(&request);
                let path = request.split_whitespace().nth(1).unwrap_or("").to_string();
                log.lock().unwrap().push(path.clone());
                let response = data.lock().unwrap().get(&path).cloned();
                let default_status = if response.is_some() {
                    "200 OK"
                } else {
                    "404 Not Found"
                };
                let status = response_statuses
                    .lock()
                    .unwrap()
                    .get(&path)
                    .cloned()
                    .unwrap_or_else(|| default_status.into());
                let headers = response_headers
                    .lock()
                    .unwrap()
                    .get(&path)
                    .cloned()
                    .unwrap_or_default();
                let body = response.unwrap_or_default();
                if conditional.load(Ordering::Relaxed)
                    && request
                        .to_lowercase()
                        .contains("if-none-match: \"fixture\"")
                {
                    write!(stream, "HTTP/1.1 304 Not Modified\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                } else {
                    write!(stream, "HTTP/1.1 {status}\r\nContent-Length: {}\r\nETag: \"fixture\"\r\n{headers}Connection: close\r\n\r\n", body.len()).unwrap();
                    stream.write_all(&body).unwrap();
                }
            }
        });
        Self {
            source,
            responses,
            requests,
            headers,
            statuses,
            revalidate,
            running,
            thread: Some(thread),
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}

fn read_request(stream: &mut std::net::TcpStream) -> std::io::Result<Vec<u8>> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut request = Vec::new();
    let mut byte = [0];
    while !request.ends_with(b"\r\n\r\n") {
        if stream.read(&mut byte)? == 0 {
            break;
        }
        request.push(byte[0]);
    }
    Ok(request)
}

#[test]
fn nonblocking_accepted_socket_reads_a_fragmented_request() {
    use std::{net::TcpStream, sync::mpsc};

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (ready, waiting) = mpsc::channel();
    let reader = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_nonblocking(true).unwrap();
        ready.send(()).unwrap();
        read_request(&mut stream).unwrap()
    });
    let mut client = TcpStream::connect(address).unwrap();
    client.write_all(b"GET /info/").unwrap();
    waiting.recv().unwrap();
    thread::sleep(Duration::from_millis(20));
    client
        .write_all(b"rake HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    assert_eq!(
        reader.join().unwrap(),
        b"GET /info/rake HTTP/1.1\r\nHost: localhost\r\n\r\n"
    );
}
