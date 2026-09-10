# 11 — GPUI desktop UI (ricerca online)

Copre: README §22, §23, §64–68, §100.

## 1. Cos'è GPUI (stato 2025–2026)

- Framework Rust **ibrido immediate+retained, GPU-accelerato**, nato per Zed. Element = wrapper imperativo flessibile; View = Entity + trait Render chiamato ogni frame.
- **Pre-1.0, breaking changes frequenti**: obbligo di pinnare la versione (README §22/§100).
- Repo: `zed-industries/zed/crates/gpui` (+ `gpui_platform`, `gpui_windows`, `gpui_macos`, `gpui_linux`, `gpui_util`). Mirror standalone `gpui-standalone/gpui`. Docs: `gpui.rs`, `docs.rs/gpui`, esempi (Hello World, Image, Input, SVG, Uniform List, Window...).

## 2. Supporto piattaforme (decisione §23)

- **Windows: ora pienamente supportato da Zed** (stable + preview settimanali). Backend: **DirectX 11** rendering + **DirectWrite** testo, Win32 windowing (`crates/gpui_windows/platform.rs`, DirectComposition VSync). Richiede GPU DX11 (`dxdiag`), driver aggiornati, accelerazione HW attiva.
- **Linux/FreeBSD:** feature `wayland` e/o `x11` in `gpui_platform` (renderer+text inclusi).
- **macOS:** Metal (storico primario).
- Zed Windows supporta WSL/SSH remoting, estensioni WASM pari alle altre piattaforme, AI pienamente.

## 3. Regole architetturali DeepskyEyes

- `deepsky-ui → GPUI` come presentation sostituibile; resto dell'app **mai** dipendente da tipi GPUI (§75).
- UI consuma stato applicativo; non possiede USB/protocollo/acquisizione/RAW/session.
- Layout iniziale (§64): header connessione, live preview + pannello camera (sensor/lens/RAW/resolution/exposure/ISO/focus/WB/zoom + LOCK), barra sequenza (023/300, 15.000 s, START/PAUSE/STOP), footer histogram/stats/diagnostics. Pagine: Dashboard/Camera/Preview/Sequence/Calibration/Sessions/Diagnostics/Settings.
- Diagnostics (§68): mostrare requested/applied RAW/transport RX/TX/dropped, mai nascondere il grezzo.

## 4. Checklist §100

OS target, stabilità backend Windows, Linux, macOS, requisiti GPU, rendering immagini, upload texture,
preview ad alta frequenza, display grandi immagini, zoom/pan, istogramma, pinning versione, policy breaking.

## Fonti

- https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md
- https://github.com/zed-industries/zed/tree/main/crates/gpui
- https://github.com/zed-industries/zed/tree/main/crates/gpui_windows
- https://gpui.rs/
- https://docs.rs/gpui/latest/gpui/index.html
- https://zed.dev/blog/zed-for-windows-is-here
- https://zed.dev/docs/windows.md
- https://github.com/gpui-standalone/gpui
