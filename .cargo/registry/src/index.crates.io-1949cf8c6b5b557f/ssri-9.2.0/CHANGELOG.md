# `ssri` Release Changelog

<a name="9.2.0"></a>
## 9.2.0 (2023-07-18)

### Features

* **from_hex:** Implemented Integrity::from_hex(), inverse of Integrity::to_hex() (#11) ([19b34fe4](https://github.com/zkat/ssri-rs/commit/19b34fe4cc2ad788398ac3a61472230a7966b5d7))

<a name="9.1.0"></a>
## 9.1.0 (2023-07-16)

### Features

* **deps:** Relax sha-1 dep constraint (#12) ([95473be7](https://github.com/zkat/ssri-rs/commit/95473be7c23b1a1e350409c7d0c6177ab663b9f9))

<a name="9.0.0"></a>
## 9.0.0 (2023-05-20)

### Features

* **xxhash:** add xxhash as a supported algorithm (#9) ([f458aa5e](https://github.com/zkat/ssri-rs/commit/f458aa5ee980e3ba7d96ee547dc05909c3fa2674))
    * **BREAKING CHANGE**: Adds a new algorithm, and makes Algorithm and Hasher `#[non_exhaustive]`.

<a name="8.1.0"></a>
## 8.1.0 (2023-04-01)

This release upgrades miette and thiserror in lockstep, bringing in syn2, in
order to avoid [potential issues with the
upgrade](https://github.com/yaahc/eyre/pull/92).

### Features

* **deps:** bump miette and thiserror ([4ecc2c08](https://github.com/zkat/ssri-rs/commit/4ecc2c0885b6221c42f8c0cf9c3d6448843adb80))

<a name="8.0.0"></a>
## 8.0.0 (2023-02-19)

### Features

* **msrv:** Set new MSRV after dep upgrades ([ac29f2c3](https://github.com/zkat/ssri-rs/commit/ac29f2c32caa1fe810f3763279e8b0a0f52c6a70))
    * **BREAKING CHANGE**: The MSRV has been increased to 1.57.0
* **traits:** Derive more traits, including Hash, and fix line endings ([9dbb1e8c](https://github.com/zkat/ssri-rs/commit/9dbb1e8c99869e19533d6abeeb6b09930c426791))
* **miette:** Add miette and derive Diagnostic for Error ([c2071f6a](https://github.com/zkat/ssri-rs/commit/c2071f6ab74ff89448b28fd016f2f090479d44a5))

### Bug Fixes

* **clippy:** appease the paperclip ([0b1eb50d](https://github.com/zkat/ssri-rs/commit/0b1eb50def2e86ba5fe0e7a6651b65452190d1e6))
* **deps:** bump digest, sha-1 and sha2 to 0.10.x ([c611c543](https://github.com/zkat/ssri-rs/commit/c611c543c6150017b259bc990820bd5ea5915670))
* **deps:** bump thiserror from 1.0.3 to 1.0.38 ([93429d8c](https://github.com/zkat/ssri-rs/commit/93429d8c496f858182889343cded2616b007357b))
* **deps:** bump serde from 1.0.92 to 1.0.152 ([0fd3812f](https://github.com/zkat/ssri-rs/commit/0fd3812f0846238aef86a3c544ba915ffe359775))
* **deps:** bump hex to from 0.3.2 to 0.4.3 ([a01e8522](https://github.com/zkat/ssri-rs/commit/a01e852295a262753a72b056cc87395622d6175a))
* **deps:** bump base64 from 0.10.1 to 0.21.0 ([bf4ec575](https://github.com/zkat/ssri-rs/commit/bf4ec57566774c698891240f3d9f489fe95ab941))
* **clippy:** resolve linting issues ([354f686e](https://github.com/zkat/ssri-rs/commit/354f686e2fea55e371f656a14f673c4fabc66993))
* **clippy:** set MSRV for clippy, too ([7fb5e90b](https://github.com/zkat/ssri-rs/commit/7fb5e90b4fcba9ffce9c7c851b7a73ddbcf32a97))
* **clippy:** update for lower clippy version ([399eafd0](https://github.com/zkat/ssri-rs/commit/399eafd04fcfe0abf22fd195341d16636c3377da))
* **clippy:** more clippy fixes (#8) ([0d6c019d](https://github.com/zkat/ssri-rs/commit/0d6c019d5e581e533aaccd1113a50a2ad4f72115))

### Miscellaneous Tasks

* **deps:** bump dev dependencies to latest ([9d9d4a67](https://github.com/zkat/ssri-rs/commit/9d9d4a6789d25cd36f4f342450a9f61368db2857))

<a name="7.0.0"></a>
## 7.0.0 (2021-09-16)

I've decided to move away from Parity for various Rust libraries, such as `ssri`.
So it can be more permissively used now!

### Features

* **license:** change license to Apache-2.0 ([dad568fb](https://github.com/zkat/ssri-rs/commit/dad568fb7d61a71b428308f279cb287e45164cb8))
    * **BREAKING CHANGE**: This is a significant licensing change. Please review.

<a name="6.0.0"></a>

## (2020-08-20)

#### Breaking Changes

- **integrity:** serialize/deserialize as a string ([6762fd53](https://github.com/zkat/ssri-rs/commit/6762fd533330e7202f9409c0971948b4eba6bd5e))

<a name="5.0.0"></a>

## 5.0.0 (2019-10-20)

#### Bug Fixes

- **integrity:** sort hashes after parse ([c9f92a00](https://github.com/zkat/ssri-rs/commit/c9f92a00cc183c65e7fd48c39fe95b09236d65da))

#### Features

- **errors:** More detailed errors ([6d5cba7e](https://github.com/zkat/ssri-rs/commit/6d5cba7ebb731a8da1717976ccc957671423bc52))
- **license:** switch license to Parity + Apache-2.0 ([adac5dc0](https://github.com/zkat/ssri-rs/commit/adac5dc04f33ac8efc3dadf7ab75c4c67bfccf5c))

#### Breaking Changes

- **errors:** More detailed errors ([6d5cba7e](https://github.com/zkat/ssri-rs/commit/6d5cba7ebb731a8da1717976ccc957671423bc52))

<a name="4.1.0"></a>

## 4.1.0 (2019-10-20)

#### Bug Fixes

- **docs:** oops, used the cacache config ([67fa5ed0](https://github.com/zkat/ssri-rs/commit/67fa5ed0a39d3d6009aa322e5c71197ab653ef5d))

#### Features

- **error:** switch from failure to thiserror crate (#3) ([0ce03070](https://github.com/zkat/ssri-rs/commit/0ce030708ce9ef52be33171612a79a3f2489af4c))

<a name="4.0.0"></a>

## 4.0.0 (2019-09-07)

#### Features

- **license:** relicense under MPL-2.0 (#2) ([9dd2b3a9](https://github.com/zkat/ssri-rs/commit/9dd2b3a97cf04266a9d99246cc8dcf2db2a6b632)

#### Breaking Changes

- **license:** relicense under MPL-2.0 (#2) ([9dd2b3a9](https://github.com/zkat/ssri-rs/commit/9dd2b3a97cf04266a9d99246cc8dcf2db2a6b632)
