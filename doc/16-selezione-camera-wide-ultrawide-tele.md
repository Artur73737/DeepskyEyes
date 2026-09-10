# 16 — Selezione camera wide / ultrawide / telephoto (ricerca + implementazione)

Copre: README §50, §50b, §30, §34, §66.

## 1. Cosa dice la ricerca

- **Pixel 8 Pro (doc/01):** tre posteriori (wide 50 MP f/1.68, ultrawide 48 MP f/1.95 con AF,
  tele 48 MP f/2.8 5x ~112mm equiv) + frontale 10.5 MP. Riferimenti focali:
  ultra ~12mm, main ~25mm, tele ~112mm equiv — da confermare con dump.
- **Camera2 (doc/08):** camere logiche + `getPhysicalCameraIds()`; la logica sceglie la fisica
  da focale/crop (es. documentato Pixel 3). Su alcune fisiche l'apertura diretta fallisce
  (`unknown camera id`): si apre la logica e si indirizza la fisica via session config.
- **Regole README:** mai ID hard-codati (§2); UI capability-driven (§34);
  requested/applied/reported (§106); validità RAW/manuale **per ottica** (§50).

## 2. Implementazione in `Rust_App/`

| Elemento | File |
|---|---|
| `LensType` wire (Unknown/MainWide/Ultrawide/Telephoto/Front) + `label()` | `deepsky-protocol/src/wire_types.rs` |
| `CameraCommand::SelectLens` | `deepsky-protocol/src/commands/camera.rs` |
| `CameraLens`, `LensSelector::select()`, `identify()` (euristica facing+focale) | `deepsky-camera/src/lens.rs` |
| `lens_facing`, `focal_equiv_mm` dal discovery | `deepsky-camera/src/discovery.rs` |
| `lens_requested/applied` | `deepsky-camera/src/configuration.rs` |
| `LensSelectorView` (solo ottiche annunciate) | `deepsky-ui/src/components/lens_selector.rs` |

Note: le soglie di `identify()` sono euristiche indicative; l'autorità è il dump.
Selezione di un'ottica non annunciata → `Err`, mai fallback silenzioso.

## 3. Da misurare sul device (Phase 0)

Tabella ID logico/fisico → wide/ultrawide/tele/front; per ogni ottica: RAW sì/no,
manuale sì/no, apertura diretta fisica possibile o solo via logica.

## Fonti

- https://developer.android.com/media/camera/camera2/multi-camera
- https://source.android.com/docs/core/camera/multi-camera
- https://store.google.com/product/pixel_8_pro_specs?hl=en-US
- `doc/01-pixel-8-pro-hardware.md`, `doc/08-logical-vs-physical-cameras.md`
