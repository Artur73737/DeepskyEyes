# 15 — Zoom e risoluzione 12MP / 50MP (ricerca online + implementazione)

Copre: README §11, §11b, §30, §34, §66.

## 1. Cosa dice la ricerca

- **Pixel 8 Pro (doc/01):** sensore main 50 MP con binning 4:1 → 12.5 MP di default;
  Pro settings con toggle **50 MP full-res** e **RAW+JPEG**. Zoom: 2x da crop del main,
  5x ottico tele, Super Res fino a 30x.
- **Camera2 (docs ufficiali):** zoom da `CONTROL_ZOOM_RATIO_RANGE`,
  `SCALER_AVAILABLE_MAX_DIGITAL_ZOOM`, `SCALER_CROP_REGION`; risoluzioni da
  `StreamConfigurationMap` (+ `MAXIMUM_RESOLUTION` per l'ultra-high-res).
  Mai hard-codare "4080×3072" o "12/50MP": la lista supportata viene dal dump.
- **Regole README:** requested/applied/reported (§106), niente fallback silenzioso (§108:
  size non annunciata → `SIZE_NOT_SUPPORTED` / `VALUE_CLAMPED`), UI capability-driven (§34).

## 2. Implementazione in `Rust_App/`

| Elemento | File |
|---|---|
| `ZoomControl` (×1000), `CropRegion`, `is_supported()` | `deepsky-camera/src/controls/zoom.rs` |
| `StreamSize` (w/h/format/binned), `ResolutionControl::request()` | `deepsky-camera/src/controls/resolution.rs` |
| `SetZoom`, `SetCrop`, `SetResolution` | `deepsky-protocol/src/commands/control.rs` |
| `resolution_requested/applied`, `zoom_ratio_x1000_requested/applied` | `deepsky-camera/src/configuration.rs` |
| `raw_sizes`, `max_digital_zoom_x1000` dal dump | `deepsky-camera/src/capabilities.rs` |
| `zoom_supported`, `full_resolution_supported` → UI availability | `deepsky-camera/src/capability_model.rs` |
| `ZoomControls`, `ResolutionControls` (con flag `clamped`) | `deepsky-ui/src/components/{zoom_controls,resolution_controls}.rs` |

## 3. Da misurare sul device (Phase 0)

Dump `raw_sizes` per camera/formato, flag binned/full, `max_digital_zoom_x1000`,
verifica clamp (es. richiesta oltre il massimo → `VALUE_CLAMPED` con applied visibile).

## Fonti

- https://store.google.com/product/pixel_8_pro_specs?hl=en-US
- https://blog.google/products-and-platforms/devices/pixel/how-to-use-pixel-8-camera-pro-controls
- https://developer.android.com/reference/android/hardware/camera2/params/StreamConfigurationMap
- https://developer.android.com/reference/android/hardware/camera2/CameraCharacteristics
- `doc/01-pixel-8-pro-hardware.md`, `doc/02-camera2-capability-discovery.md`
