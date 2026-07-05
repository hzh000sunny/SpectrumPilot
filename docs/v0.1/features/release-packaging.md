# v0.1 Release Packaging

SpectrumPilot v0.1 ships as a Windows desktop application. The UI and backend are bundled by Tauri; users should not need Node.js, Rust, Python, or a separate web deployment.

## Catalog Seed Delivery

The packaged app only bundles bootstrap metadata at:

```text
apps/desktop/src-tauri/resources/3gpp/catalog_seed/seed.json
```

That bootstrap metadata points to the pinned v0.1 catalog manifest:

```text
https://raw.githubusercontent.com/hzh000sunny/SpectrumPilot/v0.1.0/data/3gpp/catalog_seed/download-manifest.json
```

The full compact catalog remains in the repository under:

```text
data/3gpp/catalog_seed/
```

At runtime, SpectrumPilot downloads the manifest and every listed JSON file, validates file size and SHA-256, writes into a staging directory, and activates the catalog atomically. If this async install fails, 3GPP lookup still has online fallback behavior.

The compact v1 index stores one default pointer per TDoc key. If the same TDoc key appears in more than one meeting directory, the seed builder keeps the first candidate after stable meeting sorting. A future compact schema can expand this to multiple pointers if duplicate-key candidate selection becomes important for offline-only lookup.

Before creating the `v0.1.0` tag, verify the catalog seed offline:

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
