# v0.1 Release Packaging

SpectrumPilot v0.1 ships as a Windows desktop application. The UI and backend are bundled by Tauri; users should not need Node.js, Rust, Python, or a separate web deployment.

## Catalog Seed Delivery

The staged compact catalog is committed in the repository at:

```text
data/3gpp/catalog_seed/
```

Release builds bundle the same directory into Tauri resources at:

```text
apps/desktop/src-tauri/resources/3gpp/catalog_seed/
```

The resource catalog is not downloaded from GitHub Release on first launch. On startup, SpectrumPilot installs missing bundled JSON files into the internal application metadata catalog. It copies the root seed/support JSON files only when absent and copies each major seed subtree only when that target subtree is empty:

```text
manifests/
records/
indexes/
compact/
```

This keeps a new install usable without network access while protecting upgraded users: a newer app version does not overwrite a local catalog that may already include later scheduled refresh data. If a lookup is not covered by the bundled seed, the normal online fallback still runs.

The compact v1 index stores one default pointer per TDoc key. If the same TDoc key appears in more than one meeting directory, the seed builder keeps the first candidate after stable meeting sorting. A future compact schema can expand this to multiple pointers if duplicate-key candidate selection becomes important for offline-only lookup.

Before creating the `v0.1.0` tag, verify both the source seed and the Tauri resource seed offline:

```bash
python3 scripts/3gpp/verify_compact_seed.py data/3gpp/catalog_seed
python3 -m unittest scripts/3gpp/test_release_readiness.py
```

If the staged catalog must be regenerated from the existing local baseline, use the offline builder. Do not run a full 3GPP crawl as part of normal build or CI:

```bash
python3 scripts/3gpp/build_compact_seed.py \
  --source /tmp/spectrumpilot-3gpp-2024-2026-catalog \
  --target data/3gpp/catalog_seed \
  --seed-version compact-stage-seed-2026-07-05-recent-2024-2026 \
  --generated-at 2026-07-05T00:00:00Z \
  --scope "RAN/SA/CT 2024-2026 recent meeting Docs compact catalog converted from staged baseline; no network crawl during conversion" \
  --force
```

After regenerating and reviewing `data/3gpp/catalog_seed/`, sync it into `apps/desktop/src-tauri/resources/3gpp/catalog_seed/` before building the installer. This sync is a release-preparation step, not a normal CI crawl or live website fetch.

## Windows Installer

Windows packaging is configured in:

```text
apps/desktop/src-tauri/tauri.conf.json
```

The configured bundle targets are:

```text
nsis
msi
```

The NSIS installer uses the current-user install mode and downloads WebView2 through the Microsoft bootstrapper if the runtime is missing. That keeps the installer small while still supporting clean Win10/Win11 installs.

Local Linux/WSL development can run tests and frontend builds, but final Windows installer verification should be done on Windows or through GitHub Actions `windows-latest`.

## Auto Update

Tauri updater is enabled with:

```text
@tauri-apps/plugin-updater
@tauri-apps/plugin-process
tauri-plugin-updater
tauri-plugin-process
```

The update endpoint is:

```text
https://github.com/hzh000sunny/SpectrumPilot/releases/latest/download/latest.json
```

The public updater key is committed in `tauri.conf.json`. The private updater key must not be committed. For GitHub Actions release builds, configure these repository secrets:

```text
TAURI_SIGNING_PRIVATE_KEY
TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

Generate or store the release private key outside the repository, for example:

```text
$HOME/.config/SpectrumPilot/secrets/spectrumpilot-v0.1-updater.key
```

Copy the private key contents into `TAURI_SIGNING_PRIVATE_KEY` when preparing release automation. If the key was generated with a password, copy that password into `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`; otherwise leave the password secret unset.

## Release Commands

Run the local verification suite before tagging:

```bash
python3 -m unittest discover -s scripts/3gpp -p 'test_*.py'
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-3gpp-core
PATH=/home/hzh/.cargo/bin:$PATH cargo test -p spectrum-pilot-desktop --manifest-path apps/desktop/src-tauri/Cargo.toml
cd apps/desktop && npm test -- --run
cd apps/desktop && npm run build
PATH=/home/hzh/.cargo/bin:$PATH cargo fmt --all --check
git diff --check
```

Create the release tag only after the seed and packaging checks pass:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The GitHub Actions workflow `.github/workflows/windows-release.yml` builds the Windows installer and updater artifacts. Releases are created as drafts so installer artifacts can be downloaded and smoke-tested before publishing.
