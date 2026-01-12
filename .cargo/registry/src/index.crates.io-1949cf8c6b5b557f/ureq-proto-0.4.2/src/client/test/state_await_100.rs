use crate::client::Await100Result;

use super::scenario::Scenario;

#[test]
fn proceed_without_100_continue() {
    let scenario = Scenario::builder()
        .put("https://q.test")
        .header("expect", "100-continue")
        .build();

    let call = scenario.to_await_100();

    assert!(call.can_keep_await_100());

    let inner = call.inner();
    assert!(inner.should_send_body);
    assert!(inner.close_reason.is_empty());

    match call.proceed() {
        Ok(Await100Result::SendBody(_)) => {}
        _ => panic!("proceed without 100-continue should go to SendBody"),
    }
}

#[test]
fn proceed_after_100_continue() {
    let scenario = Scenario::builder()
        .put("https://q.test")
        .header("expect", "100-continue")
        .build();

    let mut call = scenario.to_await_100();

    let input = b"HTTP/1.1 100 Continue\r\n\r\n";
    let n = call.try_read_100(input).unwrap();
    assert_eq!(n, 25);

    assert!(!call.can_keep_await_100());

    let inner = call.inner();
    assert!(inner.should_send_body);
    assert!(inner.close_reason.is_empty());

    match call.proceed() {
        Ok(Await100Result::SendBody(_)) => {}
        _ => panic!("proceed after 100-continue should go to SendBody"),
    }
}

#[test]
fn proceed_after_403() {
    let scenario = Scenario::builder()
        .put("https://q.test")
        .header("expect", "100-continue")
        .build();

    let mut call = scenario.to_await_100();

    let input = b"HTTP/1.1 403 Forbidden\r\n\r\n";
    let n = call.try_read_100(input).unwrap();
    assert_eq!(n, 0);

    assert!(!call.can_keep_await_100());

    let inner = call.inner();
    assert!(!inner.should_send_body);
    assert!(!inner.close_reason.is_empty());

    match call.proceed() {
        Ok(Await100Result::RecvResponse(_)) => {}
        _ => panic!("proceed after 403 should go to RecvResponse"),
    }
}

#[test]
fn proceed_after_200() {
    let scenario = Scenario::builder()
        .put("https://q.test")
        .header("expect", "100-continue")
        .build();

    let mut call = scenario.to_await_100();

    let input = b"HTTP/1.1 200 Ok\r\n\r\n";
    let n = call.try_read_100(input).unwrap();
    assert_eq!(n, 0);

    assert!(!call.can_keep_await_100());

    let inner = call.inner();
    assert!(!inner.should_send_body);
    assert!(!inner.close_reason.is_empty());

    match call.proceed() {
        Ok(Await100Result::RecvResponse(_)) => {}
        _ => panic!("proceed after 200 should go to RecvResponse"),
    }
}

#[test]
fn proceed_after_403_with_headers() {
    let scenario = Scenario::builder()
        .put("https://q.test")
        .header("expect", "100-continue")
        .build();

    let mut call = scenario.to_await_100();

    let input = b"HTTP/1.1 403 Forbidden\r\nContent-Length: 100\r\n";
    let n = call.try_read_100(input).unwrap();
    assert_eq!(n, 0);

    assert!(!call.can_keep_await_100());

    let inner = call.inner();
    assert!(!inner.should_send_body);
    assert!(!inner.close_reason.is_empty());

    match call.proceed() {
        Ok(Await100Result::RecvResponse(_)) => {}
        _ => panic!("proceed after 403 should go to RecvResponse"),
    }
}
