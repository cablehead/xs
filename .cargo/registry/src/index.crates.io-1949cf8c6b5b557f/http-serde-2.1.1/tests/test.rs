#[test]
fn roundtrip_binary() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Wrap(
        #[serde(with = "http_serde::header_map")] HeaderMap,
    );

    use http::{HeaderMap, HeaderValue};
    let mut map = HeaderMap::new();
    map.insert("binary", HeaderValue::from_bytes(&[254,255]).unwrap());
    map.append("multi-value", HeaderValue::from_bytes(&[128,129,130,131]).unwrap());
    map.append("multi-value", HeaderValue::from_bytes(&[33,34,35]).unwrap());
    let wrapped = Wrap(map);

    let back_cbor: Wrap = serde_cbor::from_slice(&serde_cbor::to_vec(&wrapped).unwrap()).unwrap();
    let back_bin: Wrap = bincode::deserialize(&bincode::serialize(&wrapped).unwrap()).unwrap();
    let back_rmp: Wrap = rmp_serde::from_slice(&rmp_serde::to_vec(&wrapped).unwrap()).unwrap();
    let back_rmp_named: Wrap = rmp_serde::from_slice(&rmp_serde::to_vec_named(&wrapped).unwrap()).unwrap();

    assert_eq!(back_cbor.0, wrapped.0);
    assert_eq!(back_bin.0, wrapped.0);
    assert_eq!(back_rmp.0, wrapped.0);
    assert_eq!(back_rmp_named.0, wrapped.0);
}

#[test]
fn roundtrip() {
    use http::{uri::Authority, Method, StatusCode, Uri, Version};
    use http::{HeaderMap, HeaderValue};
    use std::io;

    let mut map = HeaderMap::new();
    map.insert("hey", HeaderValue::from_static("ho"));
    map.insert("foo", HeaderValue::from_static("bar"));
    map.append("multi-value", HeaderValue::from_static("multi"));
    map.append("multi-value", HeaderValue::from_static("valued"));

    #[derive(serde::Serialize, serde::Deserialize)]
    struct Wrap(
        #[serde(with = "http_serde::header_map")] HeaderMap,
        #[serde(with = "http_serde::uri")] Uri,
        #[serde(with = "http_serde::method")] Method,
        #[serde(with = "http_serde::status_code")] StatusCode,
        #[serde(with = "http_serde::authority")] Authority,
        #[serde(with = "http_serde::version")] Version,
    );

    let wrapped = Wrap(
        map,
        "http://example.com/".parse().unwrap(),
        Method::PUT,
        StatusCode::NOT_MODIFIED,
        "example.com:8080".parse().unwrap(),
        Version::HTTP_2,
    );
    let json = serde_json::to_string(&wrapped).unwrap();
    let yaml = serde_yaml::to_string(&wrapped).unwrap();
    let cbor = serde_cbor::to_vec(&wrapped).unwrap();
    let rmp = rmp_serde::to_vec(&wrapped).unwrap();
    let rmp_named = rmp_serde::to_vec_named(&wrapped).unwrap();
    let bin = bincode::serialize(&wrapped).unwrap();
    assert_eq!(
        "[{\"hey\":\"ho\",\"foo\":\"bar\",\"multi-value\":[\"multi\",\"valued\"]},\"http://example.com/\",\"PUT\",304,\"example.com:8080\",\"HTTP/2.0\"]",
        &json
    );
    assert_eq!(
        "- hey: ho\n  foo: bar\n  multi-value:\n  - multi\n  - valued\n- http://example.com/\n- PUT\n- 304\n- example.com:8080\n- HTTP/2.0\n",
        &yaml
    );
    let back_js_str: Wrap = serde_json::from_str(&json).unwrap();
    let back_js_reader: Wrap = serde_json::from_reader(io::Cursor::new(json.as_bytes())).unwrap();
    let back_yaml_str: Wrap = serde_yaml::from_str(&yaml).unwrap();
    let back_yaml_reader: Wrap = serde_yaml::from_reader(io::Cursor::new(yaml.as_bytes())).unwrap();
    let back_cbor: Wrap = serde_cbor::from_slice(&cbor).unwrap();
    let back_bin: Wrap = bincode::deserialize(&bin).unwrap();
    let back_rmp: Wrap = rmp_serde::from_slice(&rmp).unwrap();
    let back_rmp_named: Wrap = rmp_serde::from_slice(&rmp_named).unwrap();

    for back in [
        back_js_str,
        back_js_reader,
        back_yaml_str,
        back_yaml_reader,
        back_cbor,
        back_bin,
        back_rmp,
        back_rmp_named,
    ] {
        assert_eq!(back.0.get("hey").map(http::HeaderValue::as_bytes).unwrap(), b"ho");
        assert_eq!(back.0.get("foo").map(http::HeaderValue::as_bytes).unwrap(), b"bar");
        assert_eq!(
            back.0
                .get_all("multi-value")
                .iter()
                .map(|v| v.to_str().unwrap())
                .collect::<Vec<_>>()
                .as_slice(),
            &["multi", "valued"][..]
        );

        assert_eq!(&back.1.to_string(), "http://example.com/");
        assert_eq!(back.2, Method::PUT);
        assert_eq!(back.3, StatusCode::NOT_MODIFIED);
        assert_eq!(&back.4.to_string(), "example.com:8080");
        assert_eq!(format!("{:?}", back.5), "HTTP/2.0");
    }
}

#[test]
fn roundtrip_optional() {
    use http::{uri::Authority, Method, StatusCode, Uri, Version};
    use http::{HeaderMap, HeaderValue};
    use std::io;

    let mut map = HeaderMap::new();
    map.insert("hey", HeaderValue::from_static("ho"));
    map.insert("foo", HeaderValue::from_static("bar"));
    map.append("multi-value", HeaderValue::from_static("multi"));
    map.append("multi-value", HeaderValue::from_static("valued"));

    #[derive(serde::Serialize, serde::Deserialize)]
    struct WrapOpt {
        #[serde(with = "http_serde::option::header_map")]
        header_map: Option<HeaderMap>,
        #[serde(with = "http_serde::option::uri")]
        uri: Option<Uri>,
        #[serde(with = "http_serde::option::method")]
        method: Option<Method>,
        #[serde(with = "http_serde::option::status_code")]
        status_code: Option<StatusCode>,
        #[serde(with = "http_serde::option::authority")]
        authority: Option<Authority>,
        #[serde(with = "http_serde::option::version")]
        version: Option<Version>,
    }

    let wrapped = WrapOpt {
        header_map: Some(map),
        uri: Some("http://example.com/".parse().unwrap()),
        method: Some(Method::PUT),
        status_code: Some(StatusCode::NOT_MODIFIED),
        authority: Some("example.com:8080".parse().unwrap()),
        version: Some(Version::HTTP_2),
    };

    let wrapped_none = WrapOpt {
        header_map: None,
        uri: None,
        method: None,
        status_code: None,
        authority: None,
        version: None,
    };

    let json = serde_json::to_string(&wrapped).unwrap();
    let yaml = serde_yaml::to_string(&wrapped).unwrap();
    let cbor = serde_cbor::to_vec(&wrapped).unwrap();
    let rmp = rmp_serde::to_vec(&wrapped).unwrap();
    let rmp_named = rmp_serde::to_vec_named(&wrapped).unwrap();
    let bin = bincode::serialize(&wrapped).unwrap();

    assert_eq!(
        r#"{"header_map":{"hey":"ho","foo":"bar","multi-value":["multi","valued"]},"uri":"http://example.com/","method":"PUT","status_code":304,"authority":"example.com:8080","version":"HTTP/2.0"}"#,
        &json
    );
    assert_eq!(
        "header_map:\n  hey: ho\n  foo: bar\n  multi-value:\n  - multi\n  - valued\nuri: http://example.com/\nmethod: PUT\nstatus_code: 304\nauthority: example.com:8080\nversion: HTTP/2.0\n",
        &yaml
    );

    let back_js_str: WrapOpt = serde_json::from_str(&json).unwrap();
    let back_js_reader: WrapOpt = serde_json::from_reader(io::Cursor::new(json.as_bytes())).unwrap();
    let back_yaml_str: WrapOpt = serde_yaml::from_str(&yaml).unwrap();
    let back_yaml_reader: WrapOpt = serde_yaml::from_reader(io::Cursor::new(yaml.as_bytes())).unwrap();
    let back_cbor: WrapOpt = serde_cbor::from_slice(&cbor).unwrap();
    let back_bin: WrapOpt = bincode::deserialize(&bin).unwrap();
    let back_rmp: WrapOpt = rmp_serde::from_slice(&rmp).unwrap();
    let back_rmp_named: WrapOpt = rmp_serde::from_slice(&rmp_named).unwrap();

    for back in [
        back_js_str,
        back_js_reader,
        back_yaml_str,
        back_yaml_reader,
        back_cbor,
        back_bin,
        back_rmp,
        back_rmp_named,
    ] {
        assert_eq!(back.header_map.as_ref().unwrap().get("hey").map(http::HeaderValue::as_bytes).unwrap(), b"ho");
        assert_eq!(back.header_map.as_ref().unwrap().get("foo").map(http::HeaderValue::as_bytes).unwrap(), b"bar");
        assert_eq!(
            back.header_map.as_ref().unwrap()
                .get_all("multi-value")
                .iter()
                .map(|v| v.to_str().unwrap())
                .collect::<Vec<_>>()
                .as_slice(),
            &["multi", "valued"][..]
        );

        assert_eq!(&back.uri.as_ref().unwrap().to_string(), "http://example.com/");
        assert_eq!(back.method.as_ref().unwrap(), Method::PUT);
        assert_eq!(back.status_code.unwrap(), StatusCode::NOT_MODIFIED);
        assert_eq!(&back.authority.as_ref().unwrap().to_string(), "example.com:8080");
        assert_eq!(format!("{:?}", back.version.as_ref().unwrap()), "HTTP/2.0");
    }


    let json_none = serde_json::to_string(&wrapped_none).unwrap();
    let yaml_none = serde_yaml::to_string(&wrapped_none).unwrap();
    let cbor_none = serde_cbor::to_vec(&wrapped_none).unwrap();
    let rmp_none = rmp_serde::to_vec(&wrapped_none).unwrap();
    let rmp_named_none = rmp_serde::to_vec_named(&wrapped_none).unwrap();
    let bin_none = bincode::serialize(&wrapped_none).unwrap();

    assert_eq!(
        r#"{"header_map":null,"uri":null,"method":null,"status_code":null,"authority":null,"version":null}"#,
        &json_none
    );

    let back_js_str: WrapOpt = serde_json::from_str(&json_none).unwrap();
    let back_js_reader: WrapOpt = serde_json::from_reader(io::Cursor::new(json_none.as_bytes())).unwrap();
    let back_yaml_str: WrapOpt = serde_yaml::from_str(&yaml_none).unwrap();
    let back_yaml_reader: WrapOpt = serde_yaml::from_reader(io::Cursor::new(yaml_none.as_bytes())).unwrap();
    let back_cbor: WrapOpt = serde_cbor::from_slice(&cbor_none).unwrap();
    let back_bin: WrapOpt = bincode::deserialize(&bin_none).unwrap();
    let back_rmp: WrapOpt = rmp_serde::from_slice(&rmp_none).unwrap();
    let back_rmp_named: WrapOpt = rmp_serde::from_slice(&rmp_named_none).unwrap();

    for (fmt, back) in &[
        ("back_js_str", back_js_str),
        ("back_js_reader", back_js_reader),
        ("back_yaml_str", back_yaml_str),
        ("back_yaml_reader", back_yaml_reader),
        ("back_cbor", back_cbor),
        ("back_bin", back_bin),
        ("back_rmp", back_rmp),
        ("back_rmp_named", back_rmp_named),
    ] {
        assert_eq!(None, back.header_map, "{fmt}");
        assert_eq!(None, back.uri, "{fmt}");
        assert_eq!(None, back.method, "{fmt}");
        assert_eq!(None, back.status_code, "{fmt}");
        assert_eq!(None, back.authority, "{fmt}");
        assert_eq!(None, back.version, "{fmt}");
    }
}
