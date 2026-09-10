# 03 — Exposure, sensitivity, timing (ricerca online)

Copre: README §14, §15, §38, §58, §59, §92.

## 1. Controlli Camera2

- `SENSOR_EXPOSURE_TIME` (ns), `SENSOR_SENSITIVITY` (ISO sensibilità sensore), `SENSOR_FRAME_DURATION` (ns).
- Range da `SENSOR_INFO_EXPOSURE_TIME_RANGE`, `SENSOR_INFO_SENSITIVITY_RANGE`, `SENSOR_INFO_MAX_FRAME_DURATION`.
- Per controllo manuale: `CONTROL_AE_MODE = OFF` (richiede FULL o LIMITED+MANUAL_SENSOR).
  Varianti AE con priorità tempi (`ON_AUTO_FLASH` ecc.) e priorità ISO esistono ma per astro serve OFF.
- Pattern: `CONTROL_MODE = AUTO` + `CONTROL_AE_MODE = OFF` → manuale exposure; con `CONTROL_MODE = OFF` tutto manuale.
  Nota: AWB/AF in AE OFF sono device-dependent → per ripetibilità mettere AWB e AF a OFF o lock prima di AE OFF.

## 2. Requested vs Applied vs Reported (regola d'oro README §14/§106–108)

- UI/desktop deve distinguere `requested / applied / result (CaptureResult)`.
- Esempio: richiesto 20 s, applicato 15 s → **VALUE_CLAMPED**, mai mostrare 20 s come avvenuti.
- Vietato fallback silenzioso: RAW→JPEG silenzioso, 15 s→10 s mostrato come 15 s, manuale→AF silenzioso.

## 3. ISO ≠ ISO DSLR (§15)

`sensitivity` Camera2 è sensibilità sensore, non ISO marketing. Documentare come
"Sensor sensitivity requested through Camera2". Mostrare requested/applied/reported.
ITS test `test_locked_burst` verifica: stessa luminosità a ISO×exposure costante, rumore crescente con ISO.

## 4. Timing reale (§38/§59)

Distinguere: exposure time, frame duration, readout, transfer, processing, inter-frame delay.
`StreamConfigurationMap.getOutputMinFrameDuration()` + `getOutputStallDuration()` determinano il periodo minimo.
Test: 10 / 100 / 300 frame registrando requested/actual/start/end/intervallo. CTS/ITS copre EV compensation,
sensor sensitivity/exposure (`test_ev_compensation`, `test_sensitivity_burst`, `test_exposure`).

## 5. Cosa resta da misurare sul Pixel 8 Pro (§92)

Max/min exposure su wide/ultrawide/tele, step e precisione sensitivity, precisione exposure,
actual-vs-requested, frame duration/readout, stabilità su 300 frame (mean/median/variance/black-level/
saturazione/istogramma/hot-pixel). Un report forum (B4X, altro device) mostra es. range
`exposureTimeRange [22000, 100000000]ns / sensitivity [64,1600]` — **esempio non Pixel**: misurare il nostro.

## Fonti

- https://developer.android.com/reference/kotlin/android/hardware/camera2/CaptureRequest
- https://source.android.com/docs/compatibility/cts/camera-its-tests
- https://android.googlesource.com/platform/frameworks/base/+/master/core/java/android/hardware/camera2/CameraMetadata.java
- https://stackoverflow.com/questions/28293078/how-to-control-iso-manually-in-camera2-android
- https://stackoverflow.com/questions/71953493/how-to-set-exposure-on-camera2-api
- https://developer.android.com/agents/skills/camera/camerax/references/expert-blueprints
