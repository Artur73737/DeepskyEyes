# 12 — Preview, storage, sessione, diagnostica (ricerca online)

Copre: README §20, §21, §39–44, §60–63, §69–74, §101, §103, §104.

## 1. Preview separata dal RAW (§20–21, §101)

```
Camera → RAW → Disk
       → Preview (YUV/etc.) → Transport → Rust renderer → GPUI
```

Preview non deve mai forzare risoluzione/timing RAW. Funzioni attese: fit/100%/zoom/pan,
istogrammi RGB/luminanza, clipping, focus peaking/false-color, warning esposizione, crop centrale,
crosshair/grid, star detection, statistiche, frame n°, metadati, temperature, dropped indicator.
Checklist §101: formato YUV, risoluzione/FPS/banda/latenza preview, costo conversione CPU vs GPU,
possibilità RAW preview, debayer/istogramma performance.

## 2. Storage (§39–42, §60, §103–104)

- Pipeline: `Transport → Acquisition queue → Frame manager → Disk writer` asincrono; mai scrivere da callback transport in file GUI.
- Naming deterministico: `M42/lights/M42_2026-09-10T214512Z_L_0001.dng + darks/flats/bias/metadata/session.json`.
- Manifest per sessione: project/device/camera_id/started_at/exposure/frames/raw/resolution/sensitivity + versioni app/protocollo/Android, snapshot capability, config, risultati, hash, warning/error/thermal.
- Integrità: filename/size/timestamp/hash (SHA-256 consigliato) nel manifest.
- Crash recovery PC (es. crash a frame 137/300): scan, verifica file, ricostruzione, mark missing, offer resume, mai overwrite accidentale. Android-crash: transport-loss → PAUSED/FAILED → reconnect → verifica config → resume solo se sicuro.
- Throughput (§60/§103): RAW size, MB/s, latenza capture→PC, queue depth, dropped, disconnect. Il transport non deve diventare collo di bottiglia. Formula: `RAW_size × 300 + metadata/preview-cache/log/calibration + margine`. **Misurare, non assumere.**

## 3. Backpressure e code (§61–63)

Politiche esplicite per camera>disk, disk>camera, USB<>camera. Memoria limitata sempre.
`Camera producer → bounded ring buffer → Disk writer + Preview`. Se pieno: BLOCK o DROP PREVIEW,
**mai drop silenzioso di RAW scientifici** (`RAW queue lossless, Preview queue lossy`: mostra il newest).

## 4. Clock, metadati, log (§69–74)

- Due orologi → PING/timestamp/offset; futuro UTC/GPS/NTP.
- Metadati per frame: timestamp, camera/physical ID, resolution/format, exposure/sensitivity/frame/focus/WB/crop/zoom/OIS/processing/temperature/versioni.
- Log strutturati TRACE..ERROR con session/sequence/frame/request_id (es. `capture_started exposure_ns=15000000000 sensitivity=800`).
- Snapshot `capabilities.json` a inizio sessione; config versionate (`config_version=1`), mai JSON non versionato.

## Fonti

- https://developer.android.com/reference/android/hardware/camera2/params/StreamConfigurationMap (min/stall durations)
- https://source.android.com/docs/core/camera/camera-preview-stabilization
- https://developer.android.com/reference/android/hardware/camera2/DngCreator
- https://developer.android.com/training/monitoring-device-state/doze-standby (stabilità sessioni lunghe)
- https://camerasettings.com/guides/phone/google-pixel-8-pro (stime 4K MB/min, capacità per taglio storage)
