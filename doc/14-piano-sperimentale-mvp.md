# 14 — Piano sperimentale, testing, MVP (sintesi operativa dalle ricerche)

Copre: README §35–37, §49, §69, §70, §75–89, §105–108, §113–125.

## 1. Sequencer (§35–37)

```
IDLE → VALIDATING → PREPARING → RUNNING ⇄ PAUSED → COMPLETING → COMPLETED
RUNNING → ERROR → RECOVERING → RUNNING | FATAL_ERROR
```

Validazione pre-start: camera connessa/aperta, RAW/format/resolution/exposure/ISO supportati,
focus locked, storage/thermal/disk/transport/clock OK. Per 300×15 s tracciare per frame:
index, timestamp, requested/actual exposure+ISO, focus, WB, temp camera/device, transfer, path, checksum.

## 2. Lifecycle camera (§49) e data model (§105–108)

`CLOSED → OPENING → OPEN → CONFIGURING → READY ⇄ CAPTURING + ERROR/DISCONNECTED/RECOVERING`.
Oggetti: Device/Camera/PhysicalCamera/CapabilitySet/Configuration/CaptureRequest/Result/Frame(+Metadata)/Sequence(+Step)/Session/Transport-Thermal-Storage-Status/DiagnosticSnapshot.
Ogni parametro = Requested/Applied/Reported. Niente fallback silenzioso.

## 3. Testing (§77–83)

- Unit: protocol encode/decode, framing, checksum, capability parsing, range validation, state machine, naming, metadata, migration.
- Integration con MockTransport (Rust ⇄ Mock Android): connect/discover/configure/capture/transfer/sequence/failure/reconnect.
- Hardware su Pixel reale: discovery, RAW, manuale, ISO, focus, WB, resolution, preview, transfer.
- Long-duration obbligatori 1/2/4 h: memoria, temp, batteria, USB, failure, drop, storage, drift.
- Stress: 300×15 s poi 500/1000×15 s se pratico. Failure injection: USB crash, camera disconnect, disk full/fail, timeout, malformed, busy, invalid exposure/ISO/resolution, thermal, PC sleep, screen lock.

## 4. MVP (§113–116)

- **MVP:** connect → discover → capabilities → select → RAW+exposure+ISO+focus → 1 RAW → transfer → save → preview. Poi 300×15 s.
- **MVP-2:** engine, 300 frame, metadata, manifest, reconnect, recovery, thermal.
- **MVP-3:** dark/flat, calibration, histogram/stats, focus aid.
- Futuro: plate/mount/guiding/live-stack/autofocus/cataloghi/FITS/Siril/PixInsight/INDI/ASCOM-Alpaca.
- CLI prima della GUI (`discover/capabilities/connect/camera list|info/set/capture/sequence`) come primo client di integrazione; GUI non richiesta per automazione; backend astratti (`Camera/Transport/Acquisition/Storage/PreviewBackend`, `PixelCameraBackend`, futuri DSLR/astro/simulator con fake RAW/metadata/timing/failure).

## 5. Regole finali (§117–125)

Mai hard-codare capability / nascondere clamp / droppare RAW / legare transport a GUI;
GPUI isolato; Camera2 isolato; protocollo versionato; acquisizioni riproducibili; tutto validato.
Stack: Kotlin+Camera2+ImageReader+DngCreator+Service+Transport/Protocol | Rust+GPUI+Tokio+acquisition/sequencer/RAW/session/metadata/diagnostics | protocollo transport-independent versioned/framed/req-resp/event/capability-driven.
Successo = `Pixel 8 Pro → USB → Rust → GPUI` con RAW+manuali+WB fissa+preview+300×15 s senza cambi silenziosi/RAW persi, con metadati completi.
Motto: **Discover. Validate. Capture. Record everything. Assume nothing.**

## Fonti

Trasversali ai file 01–13 + README §113–125.
