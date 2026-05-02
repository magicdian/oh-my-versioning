# Releasing OMV

OMV releases use GitHub Releases as the authoritative binary artifact host and
npm as the cross-platform install channel.

Primary user install command:

```bash
npm install -g @magicdian/omv
```

The npm package version, Cargo package version, and GitHub release tag must stay
aligned. For `Cargo.toml` version `<version>`, create Git tag `v<version>` and
publish npm package `@magicdian/omv@<version>`.

## Supported Release Targets

All targets are required before npm publication:

- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-pc-windows-msvc`
- `aarch64-pc-windows-msvc`

## One-Time npm Bootstrap

The npm package must exist before npm Trusted Publishing can be configured. Do
this once from a temporary directory outside the repository. Do not commit these
files.

```bash
mkdir -p /tmp/npm-bootstrap-omv
cd /tmp/npm-bootstrap-omv
npm init --scope=@magicdian
```

Use this placeholder `package.json`:

```json
{
  "name": "@magicdian/omv",
  "version": "0.0.0",
  "description": "Reserved package for OMV CLI distribution.",
  "repository": {
    "type": "git",
    "url": "git+https://github.com/magicdian/oh-my-versioning.git"
  },
  "license": "MIT",
  "private": false
}
```

Publish the placeholder with an interactive npm login and 2FA:

```bash
npm login
npm publish --access public --tag bootstrap
```

Do not create an npm automation token for this project.

## npm Trusted Publishing Setup

After the bootstrap package exists, configure npm Trusted Publishing for
`@magicdian/omv`:

- Provider: GitHub Actions
- Repository owner: `magicdian`
- Repository name: `oh-my-versioning`
- Workflow filename: `release.yml`
- Package: `@magicdian/omv`

The release workflow and reusable npm publish workflow use OIDC:

- `.github/workflows/release.yml`
- `.github/workflows/npm-trusted-publish.yml`

`release.yml` is the workflow filename to configure in npm because it is the
caller workflow for the reusable npm publish workflow.

The workflow must not reference `NPM_TOKEN`, `NODE_AUTH_TOKEN`, or any long-lived
npm publish credential. Publishing should be allowed only through npm Trusted
Publishing/OIDC.

## Release Flow

1. Update `Cargo.toml` `version`.
2. Commit the version and changelog/docs changes on `main`.
3. Tag the release:

   ```bash
   git tag v<version>
   git push origin main
   git push origin v<version>
   ```

4. GitHub Actions runs the `Release` workflow.
5. `dist` builds and uploads all required platform artifacts to GitHub Release.
6. The reusable npm publish workflow downloads the generated
   `omv-npm-package.tar.gz` artifact and publishes it with npm OIDC.
7. Verify:

   ```bash
   npm view @magicdian/omv version
   npm install -g @magicdian/omv
   omv version
   ```
