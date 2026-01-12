# Changelog

All notable changes to iroh will be documented in this file.

## [0.35.0](https://github.com/n0-computer/iroh-metrics/compare/v0.34.0..0.35.0) - 2025-06-26

### ⛰️  Features

- Allow to encode a registry in openmetrics without EOF ([#27](https://github.com/n0-computer/iroh-metrics/issues/27)) - ([4e93211](https://github.com/n0-computer/iroh-metrics/commit/4e9321183d4dae10bf0e4a06b2daf2f0ad304da0))
- Add efficient metrics encoder that transfers schema only on change ([#28](https://github.com/n0-computer/iroh-metrics/issues/28)) - ([b41f033](https://github.com/n0-computer/iroh-metrics/commit/b41f03393dde84bde7ceac78c0d1f2f1a67d758e))

### ⚙️ Miscellaneous Tasks

- *(ci)* Add release script ([#26](https://github.com/n0-computer/iroh-metrics/issues/26)) - ([dcf91e4](https://github.com/n0-computer/iroh-metrics/commit/dcf91e4f373ebb6ae8847d846c9430eb6d4a1d13))

## [0.34.0](https://github.com/n0-computer/iroh-metrics/compare/v0.33.0..v0.34.0) - 2025-04-30

### ⛰️  Features

- Add derive for `MetricsGroupSet` ([#23](https://github.com/n0-computer/iroh-metrics/issues/23)) - ([ab86735](https://github.com/n0-computer/iroh-metrics/commit/ab867357ffd846bb2281685a4e1c9556dce847f8))

### 🚜 Refactor

- [**breaking**] Make metrics raw atomics, remove `prometheus_client` dependency ([#22](https://github.com/n0-computer/iroh-metrics/issues/22)) - ([27fb9b5](https://github.com/n0-computer/iroh-metrics/commit/27fb9b556bfb84a79841d98f10f978f1640d9e76))
- [**breaking**] Replace thiserror with snafu ([#24](https://github.com/n0-computer/iroh-metrics/issues/24)) - ([a589407](https://github.com/n0-computer/iroh-metrics/commit/a589407937d851c5e9424431b86eee24e3dddd93))

### ⚙️ Miscellaneous Tasks

- Add ignore to deny.toml ([#25](https://github.com/n0-computer/iroh-metrics/issues/25)) - ([55b432d](https://github.com/n0-computer/iroh-metrics/commit/55b432dc3bc31722501ddf5e30105db9db286d0b))
- Release - ([33ee40e](https://github.com/n0-computer/iroh-metrics/commit/33ee40ef8304244522156040d62cd5d5deed4d31))
- Release - ([017aa27](https://github.com/n0-computer/iroh-metrics/commit/017aa27ed257944c9678eede7ab6ff9ca2d8cc0e))

## [0.33.0](https://github.com/n0-computer/iroh-metrics/compare/iroh-metrics-derive-v0.1.0..v0.33.0) - 2025-04-16

### ⚙️ Miscellaneous Tasks

- Release - ([c467cfe](https://github.com/n0-computer/iroh-metrics/commit/c467cfe9c37b1f79702e3ae7d5d82d9588ac51b6))

## [iroh-metrics-derive-v0.1.0](https://github.com/n0-computer/iroh-metrics/compare/v0.32.0..iroh-metrics-derive-v0.1.0) - 2025-04-16

### 🐛 Bug Fixes

- *(ci)* Doc url ([#19](https://github.com/n0-computer/iroh-metrics/issues/19)) - ([40a8716](https://github.com/n0-computer/iroh-metrics/commit/40a87161cb69d77374c3deeebd4ec50ab015a2cf))

### 🚜 Refactor

- [**breaking**] Make global core optional, add derive macro, add `MetricsGroup` and `MetricsGroupSet` traits, reoganize modules ([#15](https://github.com/n0-computer/iroh-metrics/issues/15)) - ([90f3038](https://github.com/n0-computer/iroh-metrics/commit/90f3038760a13f0a9e445b492ff0c967c834620b))

### ⚙️ Miscellaneous Tasks

- Iroh-metrics-derive release prep - ([e79f91d](https://github.com/n0-computer/iroh-metrics/commit/e79f91d9e666fdfdedba2c9941ad66904dee7ab5))

## [0.32.0](https://github.com/n0-computer/iroh-metrics/compare/v0.31.0..v0.32.0) - 2025-02-26

### ⛰️  Features

- [**breaking**] Split up the `metrics` feature into `metrics` and `service` ([#11](https://github.com/n0-computer/iroh-metrics/issues/11)) - ([e80bf59](https://github.com/n0-computer/iroh-metrics/commit/e80bf592da22ae85499cfaf0fdf7877d02cee4f7))

### 🐛 Bug Fixes

- *(ci)* Correct the URL in the reported message ([#10](https://github.com/n0-computer/iroh-metrics/issues/10)) - ([fcfb1b2](https://github.com/n0-computer/iroh-metrics/commit/fcfb1b22c41a33a393043d6e91bbc7a71acb331e))

### ⚙️ Miscellaneous Tasks

- Remove individual repo project tracking ([#9](https://github.com/n0-computer/iroh-metrics/issues/9)) - ([3b2f23f](https://github.com/n0-computer/iroh-metrics/commit/3b2f23fce0cac00b7f7a0da7b46347f53d73b535))
- Release iroh-metrics version 0.32.0 - ([383b432](https://github.com/n0-computer/iroh-metrics/commit/383b432dd80b40821c2efcd3a32e68d6f32e0b7e))

## [0.31.0](https://github.com/n0-computer/iroh-metrics/compare/v0.30.0..v0.31.0) - 2025-01-14

### ⛰️  Features

- [**breaking**] Bump MSRV to 1.81 ([#5](https://github.com/n0-computer/iroh-metrics/issues/5)) - ([a5c251b](https://github.com/n0-computer/iroh-metrics/commit/a5c251b49926804be48888d4db5ddce64ae2defd))
- Reduce dependencies, especially with `--no-default-features` ([#7](https://github.com/n0-computer/iroh-metrics/issues/7)) - ([c39b0a6](https://github.com/n0-computer/iroh-metrics/commit/c39b0a638bc805ac696280023fef36d61d6ffc32))
- Gauge support ([#8](https://github.com/n0-computer/iroh-metrics/issues/8)) - ([2cd2e98](https://github.com/n0-computer/iroh-metrics/commit/2cd2e982bb8a1cbee0431902868783f6630ab62c))

### ⚙️ Miscellaneous Tasks

- Add project tracking ([#6](https://github.com/n0-computer/iroh-metrics/issues/6)) - ([5195ad6](https://github.com/n0-computer/iroh-metrics/commit/5195ad63e6aff87664ea135704832bf4d02b0d8a))
- Release iroh-metrics version 0.31.0 - ([9ac500a](https://github.com/n0-computer/iroh-metrics/commit/9ac500addf9408b9ee061c26703334620a3c01b7))

## [0.30.0](https://github.com/n0-computer/iroh-metrics/compare/v0.29.0..v0.30.0) - 2024-12-16

### ⛰️  Features

- CI ([#1](https://github.com/n0-computer/iroh-metrics/issues/1)) - ([a90ca3e](https://github.com/n0-computer/iroh-metrics/commit/a90ca3e9a2f4aa4103c1a23759e3a50c6ef2753b))
- [**breaking**] Introduce explicit error handling ([#3](https://github.com/n0-computer/iroh-metrics/issues/3)) - ([27e643e](https://github.com/n0-computer/iroh-metrics/commit/27e643e632881b2d30151758a0ef9ba1f894fecc))

### ⚙️ Miscellaneous Tasks

- Fixup changelog - ([dbba13e](https://github.com/n0-computer/iroh-metrics/commit/dbba13efa9f5419b37603182c3f85cdb9d26cac6))
- Fixup clippy and semver ([#2](https://github.com/n0-computer/iroh-metrics/issues/2)) - ([06fee28](https://github.com/n0-computer/iroh-metrics/commit/06fee28a8ea68d230f8309d9d532e4fe5fd8e936))
- Release iroh-metrics version 0.30.0 - ([0366a75](https://github.com/n0-computer/iroh-metrics/commit/0366a757842edd31dab30191f2fec86410ef1efe))

## [0.29.0](https://github.com/n0-computer/iroh-metrics/compare/v0.28.1..v0.29.0) - 2024-12-02

### 📚 Documentation

- Format code in doc comments ([#2895](https://github.com/n0-computer/iroh-metrics/issues/2895)) - ([cc8f348](https://github.com/n0-computer/iroh-metrics/commit/cc8f3486abf3ff48121e7df6cf9d6360fcbc3801))

### ⚙️ Miscellaneous Tasks

- Prune some deps ([#2932](https://github.com/n0-computer/iroh-metrics/issues/2932)) - ([b98f854](https://github.com/n0-computer/iroh-metrics/commit/b98f854272320325fece91e08e78daff38b73485))
- Add license - ([962bb2b](https://github.com/n0-computer/iroh-metrics/commit/962bb2bdabf03b25d9738e0dcd12dca741e4aa65))
- Add missing files - ([da71bad](https://github.com/n0-computer/iroh-metrics/commit/da71bade2565b5355b5c50ee2076c2280bb438d2))
- Fix release.toml - ([436933e](https://github.com/n0-computer/iroh-metrics/commit/436933e13418f98c019825c4e7cdc597bb55b3c8))
- Release iroh-metrics version 0.29.0 - ([e01c412](https://github.com/n0-computer/iroh-metrics/commit/e01c41298ce73f6dbd17917f1cdaf5271edf9552))
- Add changelog - ([e9577ca](https://github.com/n0-computer/iroh-metrics/commit/e9577cae8ed6925f339b1cb001b427ea62c7af1c))

### Ref

- *(iroh-metrics, iroh-relay)* Remove the UsageStatsReporter ([#2952](https://github.com/n0-computer/iroh-metrics/issues/2952)) - ([9c24cb1](https://github.com/n0-computer/iroh-metrics/commit/9c24cb1a2f5613c8a5b68f8639987019684967b1))

## [0.28.1](https://github.com/n0-computer/iroh-metrics/compare/v0.27.0..v0.28.1) - 2024-11-04

### ⛰️  Features

- Collect metrics for direct connections & add opt-in push metrics ([#2805](https://github.com/n0-computer/iroh-metrics/issues/2805)) - ([dedbd82](https://github.com/n0-computer/iroh-metrics/commit/dedbd820b0ebaabcb9ca0da840a6b6e966bb2c37))

### 🐛 Bug Fixes

- *(metrics)* Allow external crates to encode their metrics ([#2885](https://github.com/n0-computer/iroh-metrics/issues/2885)) - ([d09da2e](https://github.com/n0-computer/iroh-metrics/commit/d09da2ecdbcb0c9f05d12acdb845e2bd55f3c973))

### ⚙️ Miscellaneous Tasks

- Release - ([4405f41](https://github.com/n0-computer/iroh-metrics/commit/4405f413f800185bbc05af1dba7717bc0fcd3c6b))

## [0.27.0](https://github.com/n0-computer/iroh-metrics/compare/v0.26.0..v0.27.0) - 2024-10-21

### ⚙️ Miscellaneous Tasks

- Format imports using rustfmt ([#2812](https://github.com/n0-computer/iroh-metrics/issues/2812)) - ([b381364](https://github.com/n0-computer/iroh-metrics/commit/b3813642661bdc41144d5dfe2b5f40fb21daa4da))
- Increase version numbers and update ([#2821](https://github.com/n0-computer/iroh-metrics/issues/2821)) - ([dc0b31b](https://github.com/n0-computer/iroh-metrics/commit/dc0b31b94cc2bb444f4a2836aa05a046bad3b61f))

## [0.26.0](https://github.com/n0-computer/iroh-metrics/compare/v0.25.0..v0.26.0) - 2024-09-30

### ⚙️ Miscellaneous Tasks

- Release - ([74039ec](https://github.com/n0-computer/iroh-metrics/commit/74039ec2e25ae2aea2baabb961fe49a527017f4b))

## [0.25.0](https://github.com/n0-computer/iroh-metrics/compare/v0.24.0..v0.25.0) - 2024-09-16

### ⚙️ Miscellaneous Tasks

- Release - ([71b78d2](https://github.com/n0-computer/iroh-metrics/commit/71b78d2d5190f31875b19ae74bfda185508693fc))

## [0.24.0](https://github.com/n0-computer/iroh-metrics/compare/v0.23.0..v0.24.0) - 2024-09-02

### ⛰️  Features

- *(iroh-net)* [**breaking**] Upgrade to Quinn 0.11 and Rustls 0.23 ([#2595](https://github.com/n0-computer/iroh-metrics/issues/2595)) - ([c1ce443](https://github.com/n0-computer/iroh-metrics/commit/c1ce4437a0c2d8c6b5cf4b5f7a1381dc7bb66a18))

### ⚙️ Miscellaneous Tasks

- Release - ([dcd1aec](https://github.com/n0-computer/iroh-metrics/commit/dcd1aec1abcdf0e13f9c751b74aaf65f25456b11))

## [0.23.0](https://github.com/n0-computer/iroh-metrics/compare/v0.22.0..v0.23.0) - 2024-08-20

### ⚙️ Miscellaneous Tasks

- Release - ([e98e284](https://github.com/n0-computer/iroh-metrics/commit/e98e284ed06284ff2fee63418c9374b1f96ab623))

## [0.22.0](https://github.com/n0-computer/iroh-metrics/compare/v0.21.0..v0.22.0) - 2024-08-05

### ⚙️ Miscellaneous Tasks

- Release - ([f4f04ea](https://github.com/n0-computer/iroh-metrics/commit/f4f04eaeefed9e4f72ca86e56b5e2e6d702821e6))

## [0.21.0](https://github.com/n0-computer/iroh-metrics/compare/v0.20.0..v0.21.0) - 2024-07-22

### 🐛 Bug Fixes

- *(iroh-metrics)* Add the bind addr in errors for bind failures ([#2511](https://github.com/n0-computer/iroh-metrics/issues/2511)) - ([e0d4b24](https://github.com/n0-computer/iroh-metrics/commit/e0d4b24e8a6ff204f06a4051c8000c2095891a50))

### 🚜 Refactor

- [**breaking**] Metrics ([#2464](https://github.com/n0-computer/iroh-metrics/issues/2464)) - ([4588d29](https://github.com/n0-computer/iroh-metrics/commit/4588d29d4bf6b4c3456602f10c0a08e0e6115586))

### ⚙️ Miscellaneous Tasks

- Release - ([483ce5a](https://github.com/n0-computer/iroh-metrics/commit/483ce5a090cb30afa00c92cf574913d911c99e8f))

## [0.20.0](https://github.com/n0-computer/iroh-metrics/compare/v0.19.0..v0.20.0) - 2024-07-09

### ⚙️ Miscellaneous Tasks

- Release - ([01b3683](https://github.com/n0-computer/iroh-metrics/commit/01b36835d57e9eed9b2c53a925565541b88a3f6d))

## [0.19.0](https://github.com/n0-computer/iroh-metrics/compare/v0.18.0..v0.19.0) - 2024-06-27

### ⚙️ Miscellaneous Tasks

- Release - ([80f41cd](https://github.com/n0-computer/iroh-metrics/commit/80f41cdfffa30b9b743dce2684c93f5d676eddd1))

## [0.18.0](https://github.com/n0-computer/iroh-metrics/compare/v0.17.0..v0.18.0) - 2024-06-07

### ⚙️ Miscellaneous Tasks

- Release - ([3e487e2](https://github.com/n0-computer/iroh-metrics/commit/3e487e2f3716f7e2c61360718d979e536897b858))

## [0.17.0](https://github.com/n0-computer/iroh-metrics/compare/v0.16.0..v0.17.0) - 2024-05-24

### ⛰️  Features

- *(iroh-net)* [**breaking**] Implement http proxy support ([#2298](https://github.com/n0-computer/iroh-metrics/issues/2298)) - ([3888671](https://github.com/n0-computer/iroh-metrics/commit/388867163db071b2b7c1d0f5101f7c137d33de3c))
- [**breaking**] New quic-rpc, simlified generics, bump MSRV to 1.76 ([#2268](https://github.com/n0-computer/iroh-metrics/issues/2268)) - ([8f279f4](https://github.com/n0-computer/iroh-metrics/commit/8f279f424b55548f0b9f6c9bfffec97515961194))

### ⚙️ Miscellaneous Tasks

- Release - ([73f5797](https://github.com/n0-computer/iroh-metrics/commit/73f57970e8efb6da17355f06c13627a50688be50))

## [0.16.0](https://github.com/n0-computer/iroh-metrics/compare/v0.15.0..v0.16.0) - 2024-05-13

### 🚜 Refactor

- *(iroh)* [**breaking**] Cleanup public API ([#2263](https://github.com/n0-computer/iroh-metrics/issues/2263)) - ([4631cf9](https://github.com/n0-computer/iroh-metrics/commit/4631cf9b90150b1c4beda72c07ca4864c1cdf74a))

### ⚙️ Miscellaneous Tasks

- Release - ([8c172f6](https://github.com/n0-computer/iroh-metrics/commit/8c172f6323a5c9edd5100a5eb8ad71f8759c1c10))

## [0.15.0](https://github.com/n0-computer/iroh-metrics/compare/v0.14.0..v0.15.0) - 2024-04-29

### ⚙️ Miscellaneous Tasks

- Release - ([786c783](https://github.com/n0-computer/iroh-metrics/commit/786c783bd023a820ab95df32fec8fbeae3e687c4))

## [0.14.0](https://github.com/n0-computer/iroh-metrics/compare/v0.13.0..v0.14.0) - 2024-04-15

### ⚙️ Miscellaneous Tasks

- Release - ([2c84c36](https://github.com/n0-computer/iroh-metrics/commit/2c84c36761bff5295b323bd154e5aa34e413aa2b))

## [0.13.0](https://github.com/n0-computer/iroh-metrics/compare/v0.12.0..v0.13.0) - 2024-03-25

### ⚙️ Miscellaneous Tasks

- Release - ([e4c6064](https://github.com/n0-computer/iroh-metrics/commit/e4c6064a1144fabf82cf0e3c79bcdd198dc1080b))

## [0.12.0](https://github.com/n0-computer/iroh-metrics/compare/v0.11.0..v0.12.0) - 2023-12-20

### ⛰️  Features

- Usage metrics reporting ([#1862](https://github.com/n0-computer/iroh-metrics/issues/1862)) - ([2aa3f1b](https://github.com/n0-computer/iroh-metrics/commit/2aa3f1b92e4ffc66d684185594566085a8e353a2))

### 🚜 Refactor

- Upgrade to hyper 1.0 ([#1858](https://github.com/n0-computer/iroh-metrics/issues/1858)) - ([99c5b02](https://github.com/n0-computer/iroh-metrics/commit/99c5b02ebed570b6aa9471013d9b8fb04d4583a8))

### 🧪 Testing

- *(iroh-net)* Try fix flaky udp_blocked test - ([8f2f620](https://github.com/n0-computer/iroh-metrics/commit/8f2f620c873e3e3a5de909daacd83563bc57f613))

### ⚙️ Miscellaneous Tasks

- Release - ([0f33527](https://github.com/n0-computer/iroh-metrics/commit/0f335279a5ee00c60733c4f3b0e0f591aabfc059))

## [0.11.0](https://github.com/n0-computer/iroh-metrics/compare/v0.10.0..v0.11.0) - 2023-11-17

### ⚙️ Miscellaneous Tasks

- Update dependencies ([#1787](https://github.com/n0-computer/iroh-metrics/issues/1787)) - ([6e14bcf](https://github.com/n0-computer/iroh-metrics/commit/6e14bcf9dcdde8e8688004cc3a510198ce642cef))
- Release - ([ef41f3b](https://github.com/n0-computer/iroh-metrics/commit/ef41f3bb25222708fe842ef6cad632cc58d54f38))

## [0.10.0](https://github.com/n0-computer/iroh-metrics/compare/v0.9.0..v0.10.0) - 2023-11-08

### ⚙️ Miscellaneous Tasks

- Release - ([6d2e943](https://github.com/n0-computer/iroh-metrics/commit/6d2e943f98553e6ddbd6281965adcffafbb2fe7a))

## [0.9.0](https://github.com/n0-computer/iroh-metrics/compare/v0.8.0..v0.9.0) - 2023-10-31

### ⚙️ Miscellaneous Tasks

- Release - ([205341c](https://github.com/n0-computer/iroh-metrics/commit/205341c44694501b99b18809131e97ab875e76f9))

### Clippy

- Warn on unsused async fn ([#1743](https://github.com/n0-computer/iroh-metrics/issues/1743)) - ([30f8631](https://github.com/n0-computer/iroh-metrics/commit/30f86315ebdf953f0ed5cc7f10cd3fd3c6806c7e))

## [0.8.0](https://github.com/n0-computer/iroh-metrics/compare/v0.7.0..v0.8.0) - 2023-10-23

### 🐛 Bug Fixes

- Avoid FuturesUnordered ([#1647](https://github.com/n0-computer/iroh-metrics/issues/1647)) - ([3bae35a](https://github.com/n0-computer/iroh-metrics/commit/3bae35aef743fa45b2fcd2fe4c103cad363d2953))

### 🚜 Refactor

- *(net)* Improve derp client handling ([#1674](https://github.com/n0-computer/iroh-metrics/issues/1674)) - ([baebdb5](https://github.com/n0-computer/iroh-metrics/commit/baebdb5cd91cfe14de73ca5fff349284d93d8df4))

### ⚙️ Miscellaneous Tasks

- *(*)* Remove unused deps ([#1699](https://github.com/n0-computer/iroh-metrics/issues/1699)) - ([020800a](https://github.com/n0-computer/iroh-metrics/commit/020800afd4cd26708dfa835bf87d51a1171c6778))
- Release - ([c141fbf](https://github.com/n0-computer/iroh-metrics/commit/c141fbfae5e6a57fc9f10611e12054613469317b))

## [0.7.0](https://github.com/n0-computer/iroh-metrics/compare/v0.6.0..v0.7.0) - 2023-10-11

### ⚙️ Miscellaneous Tasks

- Release - ([9352d08](https://github.com/n0-computer/iroh-metrics/commit/9352d08af7a37bd06aa8045474919fac5546acb8))

## [0.6.0](https://github.com/n0-computer/iroh-metrics/compare/v0.6.0-alpha.1..v0.6.0) - 2023-09-25

### 🐛 Bug Fixes

- No-default-features builds ([#1522](https://github.com/n0-computer/iroh-metrics/issues/1522)) - ([4290d91](https://github.com/n0-computer/iroh-metrics/commit/4290d91c6aceabe5d197952f9b52b5c7502f0fcf))

### ⚙️ Miscellaneous Tasks

- Release - ([6b27e02](https://github.com/n0-computer/iroh-metrics/commit/6b27e02b01e2e8b4e0ce6ef1a9f75ea8a955eb52))

## [0.6.0-alpha.1](https://github.com/n0-computer/iroh-metrics/compare/v0.6.0-alpha.0..v0.6.0-alpha.1) - 2023-09-05

### ⚙️ Miscellaneous Tasks

- Release - ([50e64d7](https://github.com/n0-computer/iroh-metrics/commit/50e64d727592e52ffdd0f5f4001c946f42c9b28a))

## [0.6.0-alpha.0](https://github.com/n0-computer/iroh-metrics/compare/v0.5.1..v0.6.0-alpha.0) - 2023-08-28

### 🚜 Refactor

- *(iroh-net)* Unify key handling ([#1373](https://github.com/n0-computer/iroh-metrics/issues/1373)) - ([a6b7f19](https://github.com/n0-computer/iroh-metrics/commit/a6b7f19996e9ad71d0f05ae1b730a45dea7cd10b))

### 🧪 Testing

- Introduce iroh-test with common logging infrastructure ([#1365](https://github.com/n0-computer/iroh-metrics/issues/1365)) - ([150264e](https://github.com/n0-computer/iroh-metrics/commit/150264e40aaf3d508b9a459b1b2dfcb80d5f8e17))

### ⚙️ Miscellaneous Tasks

- Update license field following SPDX 2.1 license expression standard - ([3be0f7f](https://github.com/n0-computer/iroh-metrics/commit/3be0f7fb5346113ff92b6bdf32527af475eefd6f))
- Release - ([3eb2592](https://github.com/n0-computer/iroh-metrics/commit/3eb259201b94ef0fb17cec810411a46e4be5d2dc))

## [0.5.1](https://github.com/n0-computer/iroh-metrics/compare/xtask-v0.2.0..v0.5.1) - 2023-07-18

### ⛰️  Features

- *(iroh-net)* Upnp port mapping ([#1117](https://github.com/n0-computer/iroh-metrics/issues/1117)) - ([2d65578](https://github.com/n0-computer/iroh-metrics/commit/2d65578344a80cd9305e06d9aa579ae2c4dc700d))
- *(iroh-net)* PCP probe  - ([8f80f1e](https://github.com/n0-computer/iroh-metrics/commit/8f80f1e468229b5e0744ab102137ebabb41a2098))
- Add metrics to the derp server ([#1260](https://github.com/n0-computer/iroh-metrics/issues/1260)) - ([edd2cd7](https://github.com/n0-computer/iroh-metrics/commit/edd2cd72e792a9b4f1a46e5ca378c7b550774df4))
- Unify MSRV to 1.66 - ([28a3870](https://github.com/n0-computer/iroh-metrics/commit/28a3870cea0fb82e59906dd59165ffa6bebd70f0))

### 🚜 Refactor

- Split metrics of into its own crate - ([f6fc0a1](https://github.com/n0-computer/iroh-metrics/commit/f6fc0a139eb3089a12b38095e05538af7741fbb3))
- Pluggable metrics ([#1173](https://github.com/n0-computer/iroh-metrics/issues/1173)) - ([fbab30d](https://github.com/n0-computer/iroh-metrics/commit/fbab30da254df2f247dc305197be106a22648a9f))

### 📚 Documentation

- Deny missing docs ([#1156](https://github.com/n0-computer/iroh-metrics/issues/1156)) - ([2d2e3b1](https://github.com/n0-computer/iroh-metrics/commit/2d2e3b1ee2c4763d0b9964a72bededb156bdb740))
- Update root, iroh, iroh-metrics readmes ([#1258](https://github.com/n0-computer/iroh-metrics/issues/1258)) - ([f5a2944](https://github.com/n0-computer/iroh-metrics/commit/f5a2944cdaf91633d803573c958dd110d1445fe1))

### ⚙️ Miscellaneous Tasks

- Add metric readme and description - ([24a782a](https://github.com/n0-computer/iroh-metrics/commit/24a782a7bd9a135606930e76d98b8f5f501f73fe))
- Release - ([f03671d](https://github.com/n0-computer/iroh-metrics/commit/f03671d36216f5eeaf33a1f4030d18009ae46574))


