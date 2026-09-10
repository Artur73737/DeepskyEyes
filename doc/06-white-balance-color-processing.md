# 06 — White balance e processing controls (ricerca online)

Copre: README §18, §19.

## 1. White balance Camera2

Pipeline definita AOSP:

```
gains Bayer [R, G_even, G_odd, B]  (dominio Bayer)
  → matrice 3x3 transform (dopo demosaic, lineare → sRGB lineare)
```

- `COLOR_CORRECTION_GAINS` (RggbChannelVector), `COLOR_CORRECTION_TRANSFORM` (ColorSpaceTransform 9 valori: `r'=I0r+I1g+I2b` ecc.).
- `COLOR_CORRECTION_MODE`: `TRANSFORM_MATRIX` (usa i nostri gains+matrice, niente WB avanzato), `FAST` (no slowdown, possibile WB extra), `HIGH_QUALITY` (migliore qualità, possibile slowdown + WB extra).
- Se `CONTROL_AWB_MODE != OFF`, transform/gains app sono **ignorati/sovrascritti** (device usa ultimi valori AWB o default).
- `CONTROL_AWB_MODE`: OFF (manuale), AUTO, preset DAYLIGHT/CLOUDY/FLUORESCENT/INCANDESCENT/WARM_FLUORESCENT/SHADE/TWILIGHT.
- Novità: `COLOR_CORRECTION_COLOR_TEMPERATURE` (Kelvin) + `COLOR_CORRECTION_COLOR_TINT` (CCT mode) dove supportato.
- Nota AE OFF: AWB/AF device-dependent → lock AWB prima di passare a manuale.

Astrazione desktop (§18): `WB_AUTO / WB_PRESET / WB_TEMPERATURE / WB_TINT / WB_MANUAL_GAINS` a seconda di capability.
Lo slider Kelvin è astrazione UI, non un intero nativo Camera2.

## 2. Processing controls (§19)

`EDGE_MODE`, `NOISE_REDUCTION_MODE`, `HOT_PIXEL_MODE`, `SHADING_MODE`, `TONEMAP_MODE`, `COLOR_CORRECTION_MODE`
(+ `ABERRATION_MODE`, `DISTORTION_CORRECTION` dove presenti). Filosofia astro: **minima manipolazione**,
ma va misurato se disattivabili davvero (denoise, hot-pixel, sharpening, shading, tonemap, hidden vendor).

`MANUAL_POST_PROCESSING` garantisce controllo su tonemap/color/gain/transform; il resto va scoperto.

## Fonti

- https://developer.android.com/reference/android/hardware/camera2/CaptureRequest (COLOR_CORRECTION_*, CONTROL_AWB_MODE)
- https://android.googlesource.com/platform/system/media/+/master/camera/docs/metadata_definitions.xml
- https://stackoverflow.com/questions/72997403/android-camera2-manual-white-balance
- https://android.googlesource.com/platform/frameworks/base/+/master/core/java/android/hardware/camera2/CameraMetadata.java
