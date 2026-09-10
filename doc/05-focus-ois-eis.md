# 05 — Focus, OIS, EIS (ricerca online)

Copre: README §16, §17, §55, §56, §57, §96.

## 1. Controlli focus Camera2

- `LENS_FOCUS_DISTANCE` (diottrie, 0 = infinito), `CONTROL_AF_MODE`, `CONTROL_AF_TRIGGER`, `CONTROL_AF_REGIONS`.
- `AF_MODE_OFF` → lente controllata dall'app via `LENS_FOCUS_DISTANCE`.
- Tutti i device supportano OFF; quelli con focuser regolabile (`LENS_INFO_MINIMUM_FOCUS_DISTANCE > 0`) supportano AUTO.
- Stati AF: `FOCUSED_LOCKED / NOT_FOCUSED_LOCKED / PASSIVE_* / INACTIVE`. Il lock va fatto in AUTO prima del trigger, poi passaggio a OFF/lock.
- **Calibrazione focus** (`LENS_INFO_FOCUS_DISTANCE_CALIBRATION`): UNCALIBRATED / APPROXIMATE / CALIBRATED.
  Se non CALIBRATED, lo stesso valore può dare fuochi reali diversi (orientamento, età meccanismo, temperatura). Fondamentale per §55.

## 2. Workflow astro (§17)

```
LIVE PREVIEW → AF/manuale → fine → conferma → LOCK FOCUS → START SEQUENCE
```

Il sequencer deve rifiutare/avvisare se il focus non è locked. Vietato AF durante sequenza da 75'.

## 3. OIS (§56)

- Chiave: `LENS_OPTICAL_STABILIZATION_MODE` (ON/OFF), disponibilità in `LENS_INFO_AVAILABLE_OPTICAL_STABILIZATION`.
- OIS = spostamento rapido lente/sensore via giroscopi: stabilizza **dentro** la singola esposizione, non tra frame. Utile per snapshot, dubbio per 15 s su treppiede/tracker.
- Pratica fotografica: su tracking mount l'IS va **spento** (rischio di compensare il moto desiderato / feedback loop). Su Pixel 8 Pro OIS è su wide+tele: verificare se disattivabile per RAW e preview, e se residua deriva.
- Da misurare: OIS disponibile? controllabile? attivo durante RAW/preview? drift su lunga esposizione?

## 4. EIS / stabilizzazione video (§57)

- `CONTROL_VIDEO_STABILIZATION_MODE`: EIS post-capture con crop/zoom-in per compensare tra frame.
- Su molti OEM l'EIS dell'app stock **non è esposto** a terze parti (caso Samsung S10 documentato su Stack Overflow). Da disabilitare salvo desiderio esplicito; verificare anche `camera preview stabilization` (Android 13+) e crop dinamico.
- Per astro: tutto OFF a meno di test contrari.

## 5. Checklist focus (§96)

Risoluzione fuoco manuale, ripetibilità, posizione infinito, deriva termica, affidabilità AF su stelle,
comportamento lock, variazioni durante lunghe sequenze, movimento lente, interazione OIS.

## Fonti

- https://developer.android.com/reference/android/hardware/camera2/TotalCaptureResult
- https://android.googlesource.com/platform/frameworks/base/+/master/core/java/android/hardware/camera2/CameraMetadata.java
- https://stackoverflow.com/questions/66233908/camera2-setting-optical-stabilization-does-nothing-ois
- https://photo.stackexchange.com/questions/65356/astrophotography-image-stabilization-on-or-off
- https://source.android.com/docs/core/camera/camera-preview-stabilization
- https://www.b4x.com/android/forum/threads/solved-camera2-api-in-manual-mode.106051
- https://stackoverflow.com/questions/42127464/how-to-lock-focus-in-camera2-api-android
