# NixOS Packaging Plan for xs (cross-stream)

## Overview

This document outlines the plan to package `xs` (cross-stream) for nixpkgs with full systemd service support. This builds on the successful packaging of [http-nu](https://github.com/cablehead/http-nu) (nixpkgs PR #458947), but extends it to support running xs as a system service.

## Why This Differs from http-nu

While http-nu is packaged as a simple CLI tool, xs requires additional infrastructure:

| Aspect | http-nu | xs (cross-stream) |
|--------|---------|-------------------|
| **Type** | CLI utility | Long-running daemon |
| **State** | Stateless | Persistent data store |
| **Service** | No systemd service | Needs systemd service |
| **NixOS Module** | Not needed | Required for configuration |
| **User Management** | Runs as invoking user | Needs dedicated service account |
| **Port Binding** | Ephemeral | Optional persistent TCP exposure |

## Packaging Components

### 1. Package Definition

**Location in nixpkgs**: `pkgs/by-name/cr/cross-stream/package.nix`

The package will:
- Build the `xs` binary using `rustPlatform.buildRustPackage`
- Include the `xs.nu` Nushell module
- Set `doCheck = false` (tests require filesystem/network access, similar to http-nu)
- Support `openssl` and `pkg-config` dependencies

**Crate name**: `cross-stream`
**Binary name**: `xs`
**Current version**: `0.6.4`

See `package.nix.example` for the complete implementation.

### 2. NixOS Module

**Location in nixpkgs**: `nixos/modules/services/misc/xs.nix`

The module will provide declarative configuration for running xs as a system service.

#### Configuration Options

```nix
services.xs = {
  enable = true;
  user = "xs";              # Service user
  group = "xs";             # Service group
  dataDir = "/var/lib/xs/store";  # Data store location
  expose = ":3021";         # Optional TCP exposure (null = Unix socket only)
  package = pkgs.cross-stream;    # Package to use
};
```

#### What the Module Provides

1. **Systemd Service** (`xs.service`)
   - Runs `xs serve ${cfg.dataDir}` with optional `--expose ${cfg.expose}`
   - Auto-restarts on failure
   - Proper service dependencies
   - Security hardening (PrivateTmp, ProtectSystem, etc.)

2. **User/Group Management**
   - Auto-creates service user/group when using defaults
   - Respects existing users if specified

3. **State Directory Management**
   - Creates `dataDir` with proper ownership
   - Ensures permissions are correct
   - Handled via `systemd.tmpfiles.rules` or `StateDirectory`

4. **Environment Configuration**
   - Sets appropriate environment variables
   - Configures paths for xs operation

See `xs-module.nix.example` for the complete module structure.

### 3. Example User Configuration

Once packaged, NixOS users would configure xs like this:

```nix
# Minimal configuration (uses all defaults)
services.xs.enable = true;

# Custom configuration
services.xs = {
  enable = true;
  user = "myuser";
  dataDir = "/mnt/data/xs-store";
  expose = "127.0.0.1:3021";  # Expose on localhost only
};
```

## Open Questions for Feedback

Before implementing, we'd like your input on these design decisions:

### 1. User/Group Defaults

**Question**: Should the default service user be `xs` or `cross-stream`?

- **Option A**: `user = "xs"` (shorter, matches binary name)
- **Option B**: `user = "cross-stream"` (matches package name, more explicit)

### 2. Data Directory Location

**Question**: Preferred default location for the data store?

- **Current proposal**: `/var/lib/xs/store`
- **Alternative**: `/var/lib/cross-stream/store`
- **Rationale**: Follows NixOS convention (`/var/lib/<service>/`)

### 3. Unix Socket Configuration

**Question**: Should we expose the Unix socket path as a configuration option?

- **Current**: Socket path is implicit (xs default behavior)
- **Consideration**: Some users might want to customize socket location
- **Trade-off**: Simplicity vs. flexibility

### 4. xs.nu Integration

**Question**: Should the package automatically install xs.nu for the service user?

- **Option A**: Package includes xs.nu but users install manually
- **Option B**: Service automatically configures xs.nu in service user's environment
- **Consideration**: xs.nu is primarily for interactive use

### 5. Security Hardening

**Question**: Any specific systemd security restrictions you'd like to see?

We plan to include standard hardening:
- `PrivateTmp = true`
- `ProtectSystem = "strict"`
- `ProtectHome = true`
- `NoNewPrivileges = true`
- `ReadWritePaths = [ cfg.dataDir ]`

### 6. Network Exposure Defaults

**Question**: Should TCP exposure be opt-in (default: null) or opt-out?

- **Current proposal**: Default to Unix socket only (`expose = null`)
- **Rationale**: More secure default, user explicitly enables TCP
- **Alternative**: Default to `":3021"` for easier setup

## Implementation Plan

### Phase 1: Package (Week 1)
1. Create `package.nix` following http-nu pattern
2. Test local build in `~/nixpkgs`
3. Verify binary works and xs.nu is included

### Phase 2: NixOS Module (Week 1-2)
1. Create `nixos/modules/services/misc/xs.nix`
2. Implement options and systemd service
3. Test in NixOS VM or configuration

### Phase 3: Documentation (Week 2)
1. Add module documentation
2. Create NIXOS_PACKAGING_GUIDE.md (similar to http-nu)
3. Document example configurations

### Phase 4: Submission (Week 2-3)
1. Create nixpkgs PR with both package and module
2. Respond to reviewer feedback
3. Add maintainer (us) to both package and module

### Phase 5: Maintenance
- Monitor for new xs releases
- Keep package updated
- Respond to NixOS user issues

## Maintainer Commitment

We commit to maintaining both the package and NixOS module in nixpkgs:
- Responding to issues and PRs
- Updating for new xs releases
- Ensuring compatibility with NixOS releases
- Following nixpkgs standards and conventions

Our team has already successfully packaged and maintains http-nu in nixpkgs.

## Benefits to xs Community

1. **Easy Installation**: NixOS users can install with `nix-env -iA nixpkgs.cross-stream`
2. **Declarative Services**: System administrators can configure xs in their NixOS configs
3. **Reproducibility**: Guaranteed consistent deployments across machines
4. **Visibility**: Listed on https://search.nixos.org/packages
5. **Automated Testing**: CI testing on multiple platforms via nixpkgs
6. **Version Management**: Automated update PRs from nixpkgs bots

## References

- **http-nu packaging**: https://github.com/NixOS/nixpkgs/pull/458947
- **http-nu guide**: https://github.com/cablehead/http-nu/blob/main/NIXOS_PACKAGING_GUIDE.md
- **xs documentation**: https://cablehead.github.io/xs/
- **NixOS module docs**: https://nixos.org/manual/nixos/stable/#sec-writing-modules

## Next Steps

1. **Review this plan** and provide feedback on open questions
2. **Approve architecture** or suggest modifications
3. **Begin implementation** following your guidance
4. **Collaborate on testing** before nixpkgs submission

## Contact

We're happy to discuss any aspect of this plan. Please comment on this PR or reach out directly.

---

**Status**: 📋 Planning - Awaiting Feedback
**Last Updated**: 2025-11-06
