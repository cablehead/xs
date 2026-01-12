#![deny(
    clippy::pedantic,
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    rustdoc::broken_intra_doc_links,
    warnings
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
// Required for NEON on 32-bit ARM until stable
#![cfg_attr(
    all(feature = "nightly", target_arch = "arm"),
    feature(
        stdarch_arm_neon_intrinsics,
        stdarch_arm_feature_detection,
        arm_target_feature
    )
)]
// Required for VSX until stable
#![cfg_attr(
    all(
        feature = "nightly",
        any(target_arch = "powerpc64", target_arch = "powerpc")
    ),
    feature(
        stdarch_powerpc,
        stdarch_powerpc_feature_detection,
        powerpc_target_feature
    )
)]
// Required for s390x vectors until stable
#![cfg_attr(
    all(feature = "nightly", target_arch = "s390x"),
    feature(stdarch_s390x, stdarch_s390x_feature_detection, s390x_target_feature)
)]
// Required for LASX until stable
#![cfg_attr(
    all(feature = "nightly", target_arch = "loongarch64"),
    feature(
        stdarch_loongarch,
        stdarch_loongarch_feature_detection,
        loongarch_target_feature
    )
)]
#![doc = include_str!("../README.md")]

// If the client or server implementation is enabled, at least one SHA1 backend
// is required.
#[cfg(all(
    any(feature = "client", feature = "server"),
    not(any(
        feature = "ring",
        feature = "aws_lc_rs",
        feature = "openssl",
        feature = "sha1_smol"
    ))
))]
compile_error!("client and server implementation require at least one SHA1 backend");

#[cfg(feature = "client")]
pub mod client;
pub mod error;
mod mask;
pub mod proto;
#[cfg(feature = "client")]
mod rand;
#[cfg(feature = "client")]
pub mod resolver;
#[cfg(feature = "server")]
pub mod server;
#[cfg(any(feature = "client", feature = "server"))]
mod sha;
pub mod tls;
#[cfg(any(feature = "client", feature = "server"))]
pub mod upgrade;
mod utf8;

#[cfg(feature = "client")]
pub use client::Builder as ClientBuilder;
pub use error::Error;
pub use proto::{CloseCode, Config, Limits, Message, Payload, WebSocketStream};
#[cfg(feature = "server")]
pub use server::Builder as ServerBuilder;
pub use tls::{Connector, MaybeTlsStream};
