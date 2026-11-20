//! The store commands against the store `$env.XS_ADDR` points at.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use nu_protocol::{PipelineData, Signals, Span, Value};
use serde_json::json;
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::nu::{commands, util, Engine};
use crate::store::{Frame, Store};

fn rt() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| tokio::runtime::Runtime::new().unwrap())
}

fn new_store() -> Store {
    let _guard = rt().enter();
    Store::new(TempDir::new().unwrap().keep()).unwrap()
}

/// A store with `.cat` and `.append` bound to it.
fn local() -> (Store, Engine) {
    let store = new_store();
    let mut engine = Engine::new().unwrap();
    engine
        .add_commands(vec![
            Box::new(commands::cat_command::CatCommand::new(store.clone())),
            Box::new(commands::append_command::AppendCommand::new(store.clone())),
            Box::new(commands::last_command::LastCommand::new(store.clone())),
            Box::new(commands::get_command::GetCommand::new(store.clone())),
            Box::new(commands::remove_command::RemoveCommand::new(store.clone())),
            Box::new(commands::cas_command::CasCommand::new(store.clone())),
            Box::new(commands::cas_post_command::CasPostCommand::new(
                store.clone(),
            )),
        ])
        .unwrap();
    (store, engine)
}

/// Another store, served on its socket, as `xs serve` does.
fn remote() -> Store {
    let store = new_store();
    let served = store.clone();
    rt().spawn(async move {
        let _ = crate::api::serve(served, Engine::new().unwrap(), None).await;
    });
    // Wait until it accepts a connection: the socket file can be there
    // before the server listens on it, and a busy test run shows that gap.
    let sock = store.path.join("sock");
    let start = Instant::now();
    while std::os::unix::net::UnixStream::connect(&sock).is_err() {
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "server never listened"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    store
}

enum Conn {
    Open,
    Closed(Instant),
}

/// A unix-socket proxy in front of `upstream`'s socket, reporting each
/// connection as it opens, and the moment its client side closes.
fn proxy(upstream: &Store) -> (PathBuf, mpsc::Receiver<Conn>) {
    let path = TempDir::new().unwrap().keep().join("p.sock");
    let listener = {
        let _guard = rt().enter();
        tokio::net::UnixListener::bind(&path).unwrap()
    };
    let target = upstream.path.join("sock");
    let (tx, rx) = mpsc::channel();
    rt().spawn(async move {
        loop {
            let (mut client, _) = listener.accept().await.unwrap();
            let mut server = tokio::net::UnixStream::connect(&target).await.unwrap();
            let tx = tx.clone();
            let _ = tx.send(Conn::Open);
            tokio::spawn(async move {
                let (mut cr, mut cw) = client.split();
                let (mut sr, mut sw) = server.split();
                let (mut cbuf, mut sbuf) = ([0u8; 8192], [0u8; 8192]);
                loop {
                    tokio::select! {
                        n = cr.read(&mut cbuf) => match n {
                            Ok(0) | Err(_) => break,
                            Ok(n) => if sw.write_all(&cbuf[..n]).await.is_err() { break },
                        },
                        n = sr.read(&mut sbuf) => match n {
                            Ok(0) | Err(_) => break,
                            Ok(n) => if cw.write_all(&sbuf[..n]).await.is_err() { break },
                        },
                    }
                }
                let _ = tx.send(Conn::Closed(Instant::now()));
            });
        }
    });
    (path, rx)
}

fn eval(engine: &Engine, command: String) -> Value {
    let engine = engine.clone();
    std::thread::spawn(move || {
        engine
            .eval(PipelineData::empty(), command)
            .unwrap()
            .into_value(Span::test_data())
            .unwrap()
    })
    .join()
    .unwrap()
}

fn frame(value: Value) -> Frame {
    serde_json::from_value(util::value_to_json(&value)).unwrap()
}

fn count(engine: &Engine, command: String) -> i64 {
    eval(engine, format!("{command} | length"))
        .as_int()
        .unwrap()
}

#[test]
fn append_and_cat_go_where_xs_addr_points() {
    let (store, engine) = local();
    let other = remote();
    let at = |body: &str| {
        format!(
            r#"with-env {{XS_ADDR: "{}"}} {{ {body} }}"#,
            other.path.display()
        )
    };

    // Inside the scope: the other store gets the frame and its content.
    let f = frame(eval(&engine, at(r#""hi" | .append t --meta {a: 1}"#)));
    assert_eq!(f.topic, "t");
    assert_eq!(f.meta, Some(json!({"a": 1})));
    assert_eq!(other.cas_read_sync(&f.hash.unwrap()).unwrap(), b"hi");
    assert_eq!(count(&engine, at(".cat -T t")), 1);
    assert_eq!(count(&engine, ".cat -T t".into()), 0);

    // Outside it: the command's own store.
    let f = frame(eval(&engine, r#""here" | .append t"#.into()));
    assert_eq!(store.cas_read_sync(&f.hash.unwrap()).unwrap(), b"here");
    assert_eq!(count(&engine, ".cat -T t".into()), 1);
    assert_eq!(count(&engine, at(".cat -T t")), 1);

    // The scope's frames read back the way a local read gives them.
    let remote_frame = eval(&engine, at(".cat -T t | first"));
    let rec = remote_frame.as_record().unwrap();
    assert_eq!(rec.get("topic").unwrap().as_str().unwrap(), "t");
    assert!(rec.get("ttl").is_none(), "ttl is stripped without --detail");
}

fn try_eval(engine: &Engine, command: String) -> Result<Value, String> {
    let engine = engine.clone();
    std::thread::spawn(move || {
        let data = engine
            .eval(PipelineData::empty(), command)
            .map_err(|e| e.to_string())?;
        data.into_value(Span::test_data())
            .map_err(|e| e.to_string())
    })
    .join()
    .unwrap()
}

#[test]
fn the_other_store_commands_go_where_xs_addr_points() {
    let (store, engine) = local();
    let other = remote();
    let at = |body: &str| {
        format!(
            r#"with-env {{XS_ADDR: "{}"}} {{ {body} }}"#,
            other.path.display()
        )
    };

    // .cas-post writes to the other store's CAS, and .cas reads it back.
    let hash = eval(&engine, at(r#""hello" | .cas-post"#));
    let hash = hash.as_str().unwrap().to_string();
    assert_eq!(
        other.cas_read_sync(&hash.parse().unwrap()).unwrap(),
        b"hello"
    );
    assert!(store.cas_read_sync(&hash.parse().unwrap()).is_err());
    let content = eval(&engine, at(&format!(".cas {hash}")));
    assert_eq!(content.as_str().unwrap(), "hello");
    assert!(try_eval(&engine, format!(".cas {hash}")).is_err());

    // .last reads the other store's most recent frames.
    let first = other.append(Frame::builder("t").build()).unwrap();
    let second = other.append(Frame::builder("t").build()).unwrap();
    let last = frame(eval(&engine, at(".last t")));
    assert_eq!(last.id, second.id);
    assert_eq!(eval(&engine, at(".last t 2")).as_list().unwrap().len(), 2);
    assert!(eval(&engine, ".last t".into()).is_nothing());

    // .get finds a frame in the other store, not in this one.
    let got = frame(eval(&engine, at(&format!(".get {}", first.id))));
    assert_eq!(got.id, first.id);
    assert!(try_eval(&engine, format!(".get {}", first.id)).is_err());

    // .remove removes it from the other store.
    eval(&engine, at(&format!(".remove {}", first.id)));
    assert!(other.get(&first.id).is_none());
    assert!(other.get(&second.id).is_some());
}

#[test]
fn xs_addr_naming_the_store_itself_stays_local() {
    // No server runs for this store, so a remote call would fail.
    let (store, engine) = local();
    for addr in [store.path.clone(), store.path.join("sock")] {
        let f = frame(eval(
            &engine,
            format!(
                r#"with-env {{XS_ADDR: "{}"}} {{ "x" | .append t }}"#,
                addr.display()
            ),
        ));
        assert!(store.cas_read_sync(&f.hash.unwrap()).is_ok());
    }
    assert_eq!(count(&engine, ".cat -T t".into()), 2);

    // The other store commands too.
    let at = format!(r#"with-env {{XS_ADDR: "{}"}}"#, store.path.display());
    let hash = eval(&engine, format!(r#"{at} {{ "y" | .cas-post }}"#));
    let hash = hash.as_str().unwrap().to_string();
    assert_eq!(store.cas_read_sync(&hash.parse().unwrap()).unwrap(), b"y");
    let content = eval(&engine, format!("{at} {{ .cas {hash} }}"));
    assert_eq!(content.as_str().unwrap(), "y");
    let last = frame(eval(&engine, format!("{at} {{ .last t }}")));
    let got = frame(eval(&engine, format!("{at} {{ .get {} }}", last.id)));
    assert_eq!(got.id, last.id);
    eval(&engine, format!("{at} {{ .remove {} }}", last.id));
    assert!(store.get(&last.id).is_none());
}

#[test]
fn an_unreachable_xs_addr_is_an_error() {
    let (_store, engine) = local();
    let engine = engine.clone();
    let out = std::thread::spawn(move || {
        engine.eval(
            PipelineData::empty(),
            r#"with-env {XS_ADDR: "/nonexistent/xs/store"} { "x" | .append t }"#.into(),
        )
    })
    .join()
    .unwrap();
    let err = out.expect_err("an append to nowhere fails").to_string();
    assert!(err.contains("xs.remote"), "{err}");
}

/// Wait for the proxy's next event of this kind.
fn next_open(rx: &mpsc::Receiver<Conn>) {
    loop {
        match rx
            .recv_timeout(Duration::from_secs(5))
            .expect("no connection")
        {
            Conn::Open => return,
            Conn::Closed(_) => continue,
        }
    }
}

fn next_close(rx: &mpsc::Receiver<Conn>) -> Instant {
    loop {
        match rx
            .recv_timeout(Duration::from_secs(5))
            .expect("the connection to the remote store stayed open")
        {
            Conn::Closed(at) => return at,
            Conn::Open => continue,
        }
    }
}

// A remote follow on a quiet topic parks waiting for a frame that never
// comes. Interrupting it (a job kill, Ctrl-C) must end the read and close
// its connection to the other store, not leave it open until a frame lands.
#[test]
fn a_remote_follow_closes_its_connection_when_interrupted() {
    let (_store, mut engine) = local();
    let other = remote();
    let (sock, conns) = proxy(&other);

    let flag = Arc::new(AtomicBool::new(false));
    engine.state.set_signals(Signals::new(flag.clone()));

    let (done_tx, done_rx) = mpsc::channel();
    let command = format!(
        r#"with-env {{XS_ADDR: "{}"}} {{ .cat -f -T idle }}"#,
        sock.display()
    );
    std::thread::spawn(move || {
        let out = engine.eval(PipelineData::empty(), command);
        let _ = done_tx.send(out.map(|d| d.into_iter().count()).is_ok());
    });

    next_open(&conns);
    std::thread::sleep(Duration::from_millis(200));
    let interrupted = Instant::now();
    flag.store(true, Ordering::SeqCst);

    let closed = next_close(&conns);
    let took = closed.duration_since(interrupted);
    eprintln!("remote follow: connection closed {took:?} after the interrupt");
    assert!(took < Duration::from_secs(1), "closed only after {took:?}");
    assert!(
        done_rx.recv_timeout(Duration::from_secs(5)).is_ok(),
        "the follow ignored its interrupt",
    );
}

// The other way a follow ends: its reader goes away (an http-nu client
// hanging up drops the response stream). The connection closes then too.
#[test]
fn a_remote_follow_closes_its_connection_when_dropped() {
    let (_store, engine) = local();
    let other = remote();
    other.append(Frame::builder("seed").build()).unwrap();
    let (sock, conns) = proxy(&other);

    let command = format!(
        r#"with-env {{XS_ADDR: "{}"}} {{ .cat -f -T seed }}"#,
        sock.display()
    );
    let dropped = std::thread::spawn(move || {
        let mut frames = engine
            .eval(PipelineData::empty(), command)
            .unwrap()
            .into_iter();
        assert!(frames.next().is_some(), "expected the seeded frame");
        drop(frames);
        Instant::now()
    })
    .join()
    .unwrap();

    next_open(&conns);
    let closed = next_close(&conns);
    let took = closed.saturating_duration_since(dropped);
    eprintln!("remote follow: connection closed {took:?} after the stream was dropped");
    assert!(took < Duration::from_secs(1), "closed only after {took:?}");
}

/// `client::frames` used straight from Rust, with no nushell: what a Rust
/// program talking to another store would write.
#[test]
fn client_frames_from_rust() {
    use crate::client::frames;
    use crate::store::ReadOptions;
    use futures::StreamExt;

    let other = remote();
    let addr = other.path.display().to_string();
    rt().block_on(async {
        let first = frames::append(&addr, "note", &b"one"[..], Some(&json!({"k": 1})), None)
            .await
            .unwrap();
        let second = frames::append(&addr, "note", &b"two"[..], None, None)
            .await
            .unwrap();
        assert_eq!(first.topic, "note");
        assert_eq!(first.meta, Some(json!({"k": 1})));
        // The frame came back as the other store wrote it.
        assert_eq!(other.get(&first.id).unwrap().id, first.id);

        let got = frames::get(&addr, &second.id.to_string()).await.unwrap();
        assert_eq!(got.map(|f| f.id), Some(second.id));

        let options = ReadOptions::builder().topic("note".to_string()).build();
        let read: Vec<Frame> = frames::cat(&addr, options)
            .await
            .unwrap()
            .map(|f| f.unwrap())
            .collect()
            .await;
        assert_eq!(
            read.iter().map(|f| f.id).collect::<Vec<_>>(),
            vec![first.id, second.id]
        );

        let last: Vec<Frame> = frames::last(&addr, Some("note".to_string()), 1, false)
            .await
            .unwrap()
            .map(|f| f.unwrap())
            .collect()
            .await;
        assert_eq!(
            last.iter().map(|f| f.id).collect::<Vec<_>>(),
            vec![second.id]
        );

        let hash = frames::cas_post(&addr, &b"content"[..]).await.unwrap();
        assert_eq!(frames::cas_get(&addr, hash).await.unwrap(), b"content");

        frames::remove(&addr, &first.id.to_string()).await.unwrap();
        assert!(frames::get(&addr, &first.id.to_string())
            .await
            .unwrap()
            .is_none());
    });
}
